use mappr_common::utils::ip::{self, Ipv6AddressType};
use crate::terminal::{colors, print};
use colored::*;
use pnet::ipnetwork::IpNetwork;
use pnet::datalink::NetworkInterface;
use std::collections::BTreeSet;
use std::net::IpAddr;

pub fn to_key_value_pair(ips: &BTreeSet<IpAddr>) -> Vec<(String, ColoredString)> {
    ips.iter()
        .map(|ip| match ip {
            IpAddr::V4(ipv4_addr) => {
                let value = ipv4_addr.to_string().color(colors::IPV4_ADDR);
                (String::from("IPv4"), value)
            }
            IpAddr::V6(ipv6_addr) => {
                let ipv6_type = match ip::get_ipv6_type(ipv6_addr) {
                    Ipv6AddressType::GlobalUnicast => "GUA",
                    Ipv6AddressType::UniqueLocal => "ULA",
                    Ipv6AddressType::LinkLocal => "LLA",
                    _ => "IPv6",
                };
                let ipv6_addr = ipv6_addr.to_string().color(colors::IPV6_ADDR);
                (String::from(ipv6_type), ipv6_addr)
            }
        })
        .collect()
}

pub fn to_key_value_pair_net(ip_net: &[IpNetwork]) -> Vec<(String, ColoredString)> {
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
                let ipv6_type = ip::get_ipv6_type(&ipv6_network.ip());

                let key = match ipv6_type {
                    Ipv6AddressType::GlobalUnicast => "GUA",
                    Ipv6AddressType::LinkLocal => "LLA",
                    Ipv6AddressType::UniqueLocal => "ULA",
                    _ => "IPv6",
                };
                (key.to_string(), value)
            }
        })
        .collect()
}

pub fn print_interface(interface: &NetworkInterface, idx: usize) {
    print::tree_head(idx, &interface.name);
    let mut key_value_pair: Vec<(String, ColoredString)> = to_key_value_pair_net(&interface.ips);
    if let Some(mac_addr) = interface.mac {
        key_value_pair.push((
            "MAC".to_string(),
            mac_addr.to_string().color(colors::MAC_ADDR),
        ));
    }
    print::as_tree_one_level(key_value_pair);
}
