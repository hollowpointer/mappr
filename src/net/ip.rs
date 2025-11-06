use std::net::{IpAddr, Ipv6Addr};

#[derive(Debug, Default)]
pub enum Ipv6AddressType {
    GlobalUnicast,
    UniqueLocal,
    LinkLocal,
    Loopback,
    #[default]
    Unspecified
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpWithPrefix {
    pub ip_addr: IpAddr,
    pub prefix: u8
}

impl IpWithPrefix {
    pub fn new(ip_addr: IpAddr, prefix: u8) -> Self {
        Self { ip_addr, prefix }
    }
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