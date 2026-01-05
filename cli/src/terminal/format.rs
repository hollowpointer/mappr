use crate::terminal::colors;
use colored::*;
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv6Addr};

// Logic moved from network/ip.rs
pub fn ipv6_to_type_str(ipv6_addr: &Ipv6Addr) -> &'static str {
    if is_global_unicast(&IpAddr::V6(*ipv6_addr)) {
        return "GUA";
    }
    if ipv6_addr.is_unique_local() {
        return "ULA";
    }
    if ipv6_addr.is_unicast_link_local() {
        return "LLA";
    }
    "IPv6"
}

// Helper needed because std doesn't have is_global_unicast stable for IPv6? or it's custom logic
fn is_global_unicast(ip_addr: &IpAddr) -> bool {
    match ip_addr {
        IpAddr::V6(ipv6_addr) => {
            let first_byte = ipv6_addr.octets()[0];
            0x3F >= first_byte && first_byte >= 0x20
        }
        _ => false,
    }
}

pub fn ip_to_key_value_pair(ips: &BTreeSet<IpAddr>) -> Vec<(String, ColoredString)> {
    ips.iter()
        .map(|ip| match ip {
            IpAddr::V4(ipv4_addr) => {
                let value = ipv4_addr.to_string().color(colors::IPV4_ADDR);
                (String::from("IPv4"), value)
            }
            IpAddr::V6(ipv6_addr) => {
                let ipv6_type = ipv6_to_type_str(ipv6_addr);
                let ipv6_addr = ipv6_addr.to_string().color(colors::IPV6_ADDR);
                (String::from(ipv6_type), ipv6_addr)
            }
        })
        .collect()
}
