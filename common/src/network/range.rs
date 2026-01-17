use std::net::{IpAddr, Ipv4Addr};
use std::collections::HashSet;

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

    pub fn to_iter(&self) -> impl Iterator<Item = IpAddr> {
        let start: u32 = self.start_addr.into();
        let end: u32 = self.end_addr.into();
        (start..=end).map(|ip| IpAddr::V4(Ipv4Addr::from(ip)))
    }

    pub fn contains(&self, ip: &Ipv4Addr) -> bool {
        let start: u32 = self.start_addr.into();
        let end: u32 = self.end_addr.into();
        let ip_u32: u32 = (*ip).into();
        ip_u32 >= start && ip_u32 <= end
    }
}

pub fn cidr_range(ip: Ipv4Addr, prefix: u8) -> anyhow::Result<Ipv4Range> {
    let network = pnet::ipnetwork::Ipv4Network::new(ip, prefix)?;
    let start = network.network();
    let end = network.broadcast();
    
    Ok(Ipv4Range::new(start, end))
}


#[derive(Debug, Clone, Default)]
pub struct IpCollection {
    pub ranges: Vec<Ipv4Range>,
    pub singles: HashSet<IpAddr>,
}

impl IpCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_single(&mut self, ip: IpAddr) {
        self.singles.insert(ip);
    }

    pub fn add_range(&mut self, range: Ipv4Range) {
        self.ranges.push(range);
    }

    pub fn extend(&mut self, other: IpCollection) {
        self.ranges.extend(other.ranges);
        self.singles.extend(other.singles);
    }
    
    pub fn len_estimate(&self) -> usize {
         let mut count = self.singles.len();
         for range in &self.ranges {
             let start: u32 = range.start_addr.into();
             let end: u32 = range.end_addr.into();
             count += (end - start + 1) as usize;
         }
         count
    }

    pub fn contains(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4_addr) => {
                for range in &self.ranges {
                    if range.contains(ipv4_addr) {
                        return true;
                    }
                }
            },
            _ => { }
        }
        self.singles.contains(ip)
    }
}

impl IntoIterator for IpCollection {
    type Item = IpAddr;
    type IntoIter = std::vec::IntoIter<IpAddr>;

    fn into_iter(self) -> Self::IntoIter {
        let mut all_ips = Vec::with_capacity(self.singles.len());
        all_ips.extend(self.singles);
        for range in self.ranges {
            all_ips.extend(range.to_iter());
        }
        all_ips.into_iter()
    }
}

impl FromIterator<IpCollection> for IpCollection {
    fn from_iter<I: IntoIterator<Item = IpCollection>>(iter: I) -> Self {
        let mut master = IpCollection::new();
        for collection in iter {
            master.extend(collection);
        }
        master
    }
}
