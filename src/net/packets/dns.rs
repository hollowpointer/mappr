use std::net::IpAddr;

use anyhow::Context;
use pnet::packet::dns::{DnsClass, DnsQuery, DnsTypes, MutableDnsPacket, Opcode, Retcode};

use crate::net::{ip, utils::DNS_HDR_LEN};

pub fn create_ptr_packet(ip_addr: IpAddr) -> anyhow::Result<Vec<u8>> {
    let query: DnsQuery = create_ptr_query(ip_addr)?;
    let q_fixed_len = 4;
    let qlen = query.qname.len() + q_fixed_len;
    let total = DNS_HDR_LEN + qlen;
    let mut buffer: Vec<u8> = vec![0u8; total];
    {
        let mut dns: MutableDnsPacket = MutableDnsPacket::new(&mut buffer).context("creating dns header")?;
        dns.set_id(0);
        dns.set_is_response(0);
        dns.set_opcode(Opcode::StandardQuery);
        dns.set_is_authoriative(0);
        dns.set_is_truncated(0);
        dns.set_is_recursion_desirable(1);
        dns.set_is_recursion_available(0);
        dns.set_zero_reserved(0);
        dns.set_is_non_authenticated_data(1);
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

    let type_bytes = query.qtype.0.to_be_bytes();
    buffer[cursor..cursor + 2].copy_from_slice(&type_bytes);
    cursor += 2;

    let class_bytes = query.qclass.0.to_be_bytes();
    buffer[cursor..cursor + 2].copy_from_slice(&class_bytes);
    
    Ok(Vec::from(buffer))
}

fn create_ptr_query(ip_addr: IpAddr) -> anyhow::Result<DnsQuery> {
    let zone = match ip_addr {
        IpAddr::V4(_) => "IN-ADDR.ARPA",
        IpAddr::V6(_) => "ip6.arpa",
    };
    
    let ptr_name = format!("{}.{}", ip::reverse_address(ip_addr), zone);
    let qname = encode_dns_name(&ptr_name);
    let query = DnsQuery { 
        qname,
        qtype: DnsTypes::PTR, 
        qclass: DnsClass(1), 
        payload: Vec::new()
    };
    Ok(query)
}

fn encode_dns_name(name: &str) -> Vec<u8> {
    let mut encoded = Vec::new();
    for label in name.split('.') {
        if label.is_empty() { continue; }
        encoded.push(label.len() as u8);
        encoded.extend_from_slice(label.as_bytes());
    }
    encoded.push(0);
    encoded
}