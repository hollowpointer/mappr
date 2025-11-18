use std::net::IpAddr;

use anyhow::Context;
use pnet::packet::dns::{DnsClass, DnsQuery, DnsTypes, MutableDnsPacket, Opcode, Retcode};

use crate::net::{ip, utils::DNS_HDR_LEN};

pub fn create_ptr_packet(ip_addr: IpAddr) -> anyhow::Result<Vec<u8>> {
    let mut buffer: [u8; DNS_HDR_LEN] = [0u8; DNS_HDR_LEN];
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
        
        let query: DnsQuery = create_ptr_query(ip_addr)?;
        //dns.set_queries(&[query]);
    }
    Ok(Vec::from(buffer))
}

fn create_ptr_query(ip_addr: IpAddr) -> anyhow::Result<DnsQuery> {
    let zone: &str = match ip_addr {
        IpAddr::V4(_) => "IN-ADDR.ARPA",
        IpAddr::V6(_) => "ip6.arpa",
    };
    let qname: Vec<u8> = format!("{}.{}", ip::reverse_address(ip_addr), zone).as_bytes().to_vec();
    let query: DnsQuery = DnsQuery { 
        qname,
        qtype: DnsTypes::PTR, 
        qclass: DnsClass(1), 
        payload: Vec::new()
    };
    Ok(query)
}