use std::net::Ipv4Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ipv4Range {
    pub start_addr: Ipv4Addr,
    pub end_addr: Ipv4Addr,
}

impl Ipv4Range {
    pub fn new(start_addr: Ipv4Addr, end_addr: Ipv4Addr) -> Self {
        Self {
            start_addr,
            end_addr,
        }
    }

    pub fn to_iter(&self) -> impl Iterator<Item = std::net::IpAddr> {
        let start: u32 = self.start_addr.into();
        let end: u32 = self.end_addr.into();
        (start..=end).map(|ip| std::net::IpAddr::V4(Ipv4Addr::from(ip)))
    }
}

pub fn cidr_range(ip: Ipv4Addr, prefix: u8) -> anyhow::Result<Ipv4Range> {
    let network = pnet::ipnetwork::Ipv4Network::new(ip, prefix)?;
    let start = network.network();
    let end = network.broadcast();
    
    Ok(Ipv4Range::new(start, end))
}
