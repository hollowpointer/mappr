use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct IpServiceGroup {
    pub ip_addr: IpAddr,
    pub tcp_services: Vec<Service>,
    pub udp_services: Vec<Service>,
}

impl IpServiceGroup {
    pub fn new(ip_addr: IpAddr, tcp_services: Vec<Service>, udp_services: Vec<Service>) -> Self {
        Self {
            ip_addr,
            tcp_services,
            udp_services,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub local_addr: IpAddr,
    pub local_ports: HashSet<u16>,
}

impl Service {
    pub fn new(name: String, local_addr: IpAddr, local_ports: HashSet<u16>) -> Self {
        Self {
            name,
            local_addr,
            local_ports,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirewallStatus {
    Active,
    Inactive,
    NotDetected,
}
