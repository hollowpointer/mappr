use std::{collections::BTreeSet, net::{IpAddr, Ipv6Addr}};
use colored::*;
use crate::utils::colors;

#[derive(Debug, Default)]
pub enum Ipv6AddressType {
    GlobalUnicast,
    UniqueLocal,
    LinkLocal,
    Loopback,
    #[default]
    Unspecified
}

pub fn get_ipv6_type(ipv6_addr: &Ipv6Addr) -> Ipv6AddressType {
    match true {
        _ if is_global_unicast(&ipv6_addr)      => Ipv6AddressType::GlobalUnicast,
        _ if ipv6_addr.is_unique_local()        => Ipv6AddressType::UniqueLocal,
        _ if ipv6_addr.is_unicast_link_local()  => Ipv6AddressType::LinkLocal,
        _ if ipv6_addr.is_loopback()            => Ipv6AddressType::Loopback,
        _                                       => Ipv6AddressType::Unspecified    
    }
}

pub fn is_global_unicast(ipv6_addr: &Ipv6Addr) -> bool {
    let first_byte = ipv6_addr.octets()[0];
    0x3F >= first_byte && first_byte >= 0x20
}

pub fn is_private(ip_addr: IpAddr) -> bool {
    match ip_addr {
        IpAddr::V4(ipv4) => ipv4.is_private(),
        IpAddr::V6(ipv6) => { ipv6.is_unicast_link_local() || ipv6.is_unique_local() }
    }
}

pub fn to_key_value_pair(ips: &BTreeSet<IpAddr>) -> Vec<(String, ColoredString)> {
    ips.iter().map(|ip| {
        match ip {
            IpAddr::V4(ipv4_addr) => {
                let value = ipv4_addr.to_string().color(colors::IPV4_ADDR);
                (String::from("IPv4"), value)
            },
            IpAddr::V6(ipv6_addr) => {
                let ipv6_type = match get_ipv6_type(ipv6_addr) {
                    Ipv6AddressType::GlobalUnicast  => "GUA",
                    Ipv6AddressType::UniqueLocal    => "ULA",
                    Ipv6AddressType::LinkLocal      => "LLA",
                    _                               => "IPv6"
                };
                let ipv6_addr = ipv6_addr.to_string().color(colors::IPV6_ADDR);
                (String::from(ipv6_type), ipv6_addr)
            },
        }
    }).collect()
}