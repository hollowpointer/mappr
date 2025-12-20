use std::net::{IpAddr, Ipv4Addr};

use anyhow::Context;
use pnet::packet::dns::{
    DnsClass, DnsPacket, DnsQuery, DnsTypes, MutableDnsPacket, Opcode, Retcode,
};

use crate::utils::ip;
use crate::adapters::outbound::network::utils::DNS_HDR_LEN;

pub fn get_hostname(payload: &[u8]) -> anyhow::Result<Option<(u16, String)>> {
    let dns: DnsPacket =
        DnsPacket::new(payload).ok_or_else(|| anyhow::anyhow!("Failed to parse DNS packet"))?;
    let (transaction_id, hostname_res) = dns
        .get_responses()
        .iter()
        .find_map(|response| {
            if response.rtype == DnsTypes::PTR {
                let hostname: String = decode_dns_name(&response.data)?;
                let transaction_id: u16 = u16::from_be_bytes([payload[0], payload[1]]);
                Some((transaction_id, hostname))
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No PTR record found"))?;
    Ok(Some((transaction_id, hostname_res)))
}

pub fn get_dns_server_socket_addr(ip_addr: &IpAddr) -> anyhow::Result<(IpAddr, u16)> {
    let dst_port: u16 = 53; // Needs improvement
    if ip::is_private(&ip_addr) {
        let gateway_addr: IpAddr = ip::get_gateway_addr(&ip_addr);
        return Ok((gateway_addr, dst_port));
    }
    let dst_addr: IpAddr = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
    return Ok((dst_addr, dst_port));
}

pub fn create_ptr_packet(ip_addr: &IpAddr, id: u16) -> anyhow::Result<Vec<u8>> {
    let query: DnsQuery = create_ptr_query(ip_addr)?;
    let q_fixed_len: usize = 4;
    let qlen: usize = query.qname.len() + q_fixed_len;
    let total: usize = DNS_HDR_LEN + qlen;
    let mut buffer: Vec<u8> = vec![0u8; total];

    {
        let mut dns: MutableDnsPacket =
            MutableDnsPacket::new(&mut buffer).context("creating dns header")?;
        dns.set_id(id);
        dns.set_is_response(0);
        dns.set_opcode(Opcode::StandardQuery);
        dns.set_is_authoriative(0);
        dns.set_is_truncated(0);
        dns.set_is_recursion_desirable(1);
        dns.set_is_recursion_available(0);
        dns.set_zero_reserved(0);
        dns.set_is_non_authenticated_data(0);
        dns.set_rcode(Retcode::NoError);
        dns.set_query_count(1);
        dns.set_response_count(0);
        dns.set_authority_rr_count(0);
        dns.set_additional_rr_count(0);
    }

    // Manually Write the Query Bytes into the buffer
    let mut cursor: usize = DNS_HDR_LEN;

    buffer[cursor..cursor + query.qname.len()].copy_from_slice(&query.qname);
    cursor += query.qname.len();

    let type_bytes: [u8; _] = query.qtype.0.to_be_bytes();
    buffer[cursor..cursor + 2].copy_from_slice(&type_bytes);
    cursor += 2;

    let class_bytes: [u8; _] = query.qclass.0.to_be_bytes();
    buffer[cursor..cursor + 2].copy_from_slice(&class_bytes);

    Ok(Vec::from(buffer))
}

fn create_ptr_query(ip_addr: &IpAddr) -> anyhow::Result<DnsQuery> {
    let ptr_string: String = ip::reverse_address_to_ptr(ip_addr);
    let qname: Vec<u8> = encode_dns_name(&ptr_string);
    let query: DnsQuery = DnsQuery {
        qname,
        qtype: DnsTypes::PTR,
        qclass: DnsClass(1),
        payload: Vec::new(),
    };
    Ok(query)
}

fn encode_dns_name(name: &str) -> Vec<u8> {
    let mut encoded: Vec<u8> = Vec::new();
    for label in name.split('.') {
        if label.is_empty() {
            continue;
        }
        encoded.push(label.len() as u8);
        encoded.extend_from_slice(label.as_bytes());
    }
    encoded.push(0);
    encoded
}

fn decode_dns_name(data: &[u8]) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    let mut cursor: usize = 0;
    while cursor < data.len() {
        let len: usize = data[cursor] as usize;
        if len == 0 {
            break;
        }
        cursor += 1;
        if cursor + len > data.len() {
            return None;
        }
        let label_bytes: &[u8] = &data[cursor..cursor + len];
        let label: &str = std::str::from_utf8(label_bytes).ok()?;
        parts.push(label);
        cursor += len;
    }
    Some(parts.join("."))
}
