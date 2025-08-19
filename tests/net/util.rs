use pnet::datalink::{MacAddr, NetworkInterface};
use pnet::ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use std::net::{Ipv4Addr, Ipv6Addr};

pub fn ni(name: &str, index: u32, mac: Option<MacAddr>, ips: &[IpNetwork], flags: u32) -> NetworkInterface {
    NetworkInterface {
        name: name.into(),
        description: "".into(),
        index,
        mac,
        ips: ips.to_vec(),
        flags,
    }
}

pub fn v4(a: u8,b: u8,c: u8,d: u8, p: u8) -> IpNetwork {
    IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(a,b,c,d), p).unwrap())
}
pub fn v6(s: &str, p: u8) -> IpNetwork {
    IpNetwork::V6(Ipv6Network::new(s.parse::<Ipv6Addr>().unwrap(), p).unwrap())
}