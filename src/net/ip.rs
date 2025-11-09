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