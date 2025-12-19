use colored::*;
use crate::adapters::outbound::terminal::colors;
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv6Addr};
use pnet::ipnetwork::IpNetwork;

// Logic moved from network/ip.rs
pub fn ipv6_to_type_str(ipv6_addr: &Ipv6Addr) -> &'static str {
    if is_global_unicast(&IpAddr::V6(*ipv6_addr)) { return "GUA"; }
    if ipv6_addr.is_unique_local() { return "ULA"; }
    if ipv6_addr.is_unicast_link_local() { return "LLA"; }
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

// Used by info.rs? Check usage. 
// Actually to_key_value_pair_net was in ip.rs too.
pub fn net_to_key_value_pair(ip_net: &[IpNetwork]) -> Vec<(String, ColoredString)> {
    ip_net
        .iter()
        .map(|ip_network| match ip_network {
            IpNetwork::V4(ipv4_network) => {
                let address: ColoredString = ipv4_network.ip().to_string().color(colors::IPV4_ADDR);
                let prefix: ColoredString =
                    ipv4_network.prefix().to_string().color(colors::IPV4_PREFIX);
                let result: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR);
                ("IPv4".to_string(), result)
            }
            IpNetwork::V6(ipv6_network) => {
                let address: ColoredString = ipv6_network.ip().to_string().color(colors::IPV6_ADDR);
                let prefix: ColoredString =
                    ipv6_network.prefix().to_string().color(colors::IPV6_PREFIX);
                let value: ColoredString = format!("{address}/{prefix}").color(colors::SEPARATOR);
                let ipv6_type = ipv6_to_type_str(&ipv6_network.ip());
                (ipv6_type.to_string(), value)
            }
        })
        .collect()
}
