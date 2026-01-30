use std::net::IpAddr;

use anyhow::Context;
use pnet::packet::dns::{
    DnsClass, DnsPacket, DnsQuery, DnsResponse, DnsTypes, MutableDnsPacket, Opcode, Retcode,
};

use zond_common::utils::ip;

pub const DNS_HDR_LEN: usize = 12;

pub fn get_hostname(payload: &[u8]) -> anyhow::Result<(u16, String)> {
    let dns = DnsPacket::new(payload).context("Failed to parse DNS packet")?;
    let transaction_id = dns.get_id();
    let hostname_res = dns
        .get_responses()
        .iter()
        .find_map(|response| match response.rtype {
            DnsTypes::PTR => response_from_ptr(response),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("No valid A or PTR record found"))?;

    Ok((transaction_id, hostname_res))
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

    Ok(buffer)
}

fn response_from_ptr(response: &DnsResponse) -> Option<String> {
    decode_dns_name(&response.data)
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
