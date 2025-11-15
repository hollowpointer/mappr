use anyhow;
use pnet::ipnetwork::Ipv4Network;
use std::net::Ipv4Addr;
use std::str::FromStr;

/// Represents a continuous range of IPv4 addresses, inclusive.
#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = Ipv4Addr> + Clone {
        let start = u32::from(self.start_addr);
        let end = u32::from(self.end_addr);
        (start..=end).map(Ipv4Addr::from)
    }

    pub fn contains(&self, addr: &Ipv4Addr) -> bool {
        (self.start_addr..=self.end_addr).contains(addr)
    }
}


pub fn from_ipv4_net(ipv4_net: Option<Ipv4Network>) -> Option<Ipv4Range> {
    ipv4_net.and_then(|net| cidr_range(net.ip(), net.prefix()).ok())
}


pub fn cidr_range(ip: Ipv4Addr, prefix: u8) -> anyhow::Result<Ipv4Range> {
    if prefix > 32 {
        anyhow::bail!("Invalid prefix: {prefix} > 32");
    }
    let ip_u32 = u32::from(ip);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let network = ip_u32 & mask;
    let broadcast = network | !mask;
    Ok(Ipv4Range::new(Ipv4Addr::from(network),Ipv4Addr::from(broadcast)))
}


pub fn _from_cidr_str(cidr: &str) -> anyhow::Result<Ipv4Range> {
    if !cidr.contains('/') {
        anyhow::bail!("Invalid CIDR string '{cidr}': missing '/' separator");
    }
    let net = Ipv4Network::from_str(cidr)
        .map_err(|e| anyhow::anyhow!("Invalid CIDR string '{cidr}': {e}"))?;
    cidr_range(net.ip(), net.prefix())
}


pub fn in_optional_range(addr: &Ipv4Addr, range: &Option<Ipv4Range>) -> bool {
    range
        .as_ref()
        .map_or(true, |ipv4_range| ipv4_range.contains(addr))
}



// ╔════════════════════════════════════════════╗
// ║ ████████╗███████╗███████╗████████╗███████╗ ║
// ║ ╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝ ║
// ║    ██║   █████╗  ███████╗   ██║   ███████╗ ║
// ║    ██║   ██╔══╝  ╚════██║   ██║   ╚════██║ ║
// ║    ██║   ███████╗███████║   ██║   ███████║ ║
// ║    ╚═╝   ╚══════╝╚══════╝   ╚═╝   ╚══════╝ ║
// ╚════════════════════════════════════════════╝

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_ipv4range_new() {
        let start = Ipv4Addr::new(192, 168, 0, 1);
        let end = Ipv4Addr::new(192, 168, 0, 255);
        let range = Ipv4Range::new(start, end);
        assert_eq!(range.start_addr, start);
        assert_eq!(range.end_addr, end);
    }

    #[test]
    fn test_ipv4range_iter() {
        let start = Ipv4Addr::new(10, 0, 0, 1);
        let end = Ipv4Addr::new(10, 0, 0, 3);
        let range = Ipv4Range::new(start, end);
        
        let mut iter = range.iter();
        assert_eq!(iter.next(), Some(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(iter.next(), Some(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(iter.next(), Some(Ipv4Addr::new(10, 0, 0, 3)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_ipv4range_iter_count() {
        let start = Ipv4Addr::new(10, 0, 0, 1);
        let end = Ipv4Addr::new(10, 0, 0, 255);
        let range = Ipv4Range::new(start, end);
        assert_eq!(range.iter().count(), 255);
    }

    #[test]
    fn test_ipv4range_iter_empty() {
        // Start > End
        let start = Ipv4Addr::new(10, 0, 0, 5);
        let end = Ipv4Addr::new(10, 0, 0, 1);
        let range = Ipv4Range::new(start, end);
        assert_eq!(range.iter().count(), 0);
        assert_eq!(range.iter().next(), None);
    }

    #[test]
    fn test_ipv4range_iter_single() {
        let start = Ipv4Addr::new(10, 0, 0, 1);
        let end = Ipv4Addr::new(10, 0, 0, 1);
        let range = Ipv4Range::new(start, end);
        assert_eq!(range.iter().count(), 1);
        assert_eq!(range.iter().next(), Some(start));
    }

    #[test]
    fn test_ipv4range_iter_double_ended() {
        let start = Ipv4Addr::new(10, 0, 0, 1);
        let end = Ipv4Addr::new(10, 0, 0, 3);
        let range = Ipv4Range::new(start, end);
        
        let mut iter = range.iter();
        assert_eq!(iter.next_back(), Some(Ipv4Addr::new(10, 0, 0, 3)));
        assert_eq!(iter.next(), Some(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(iter.next_back(), Some(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn test_ipv4range_contains() {
        let start = Ipv4Addr::new(192, 168, 1, 10);
        let end = Ipv4Addr::new(192, 168, 1, 20);
        let range = Ipv4Range::new(start, end);

        assert!(range.contains(&Ipv4Addr::new(192, 168, 1, 10)));
        assert!(range.contains(&Ipv4Addr::new(192, 168, 1, 15)));
        assert!(range.contains(&Ipv4Addr::new(192, 168, 1, 20)));
        
        assert!(!range.contains(&Ipv4Addr::new(192, 168, 1, 9)));
        assert!(!range.contains(&Ipv4Addr::new(192, 168, 1, 21)));
        assert!(!range.contains(&Ipv4Addr::new(192, 168, 0, 15)));
    }

    #[test]
    fn test_cidr_range() {
        let ip = Ipv4Addr::new(192, 168, 1, 100);
        let prefix = 24;
        let range = cidr_range(ip, prefix).unwrap();
        
        let expected_start = Ipv4Addr::new(192, 168, 1, 0);
        let expected_end = Ipv4Addr::new(192, 168, 1, 255);
        
        assert_eq!(range.start_addr, expected_start);
        assert_eq!(range.end_addr, expected_end);
        assert_eq!(range, Ipv4Range::new(expected_start, expected_end));
    }

    #[test]
    fn test_cidr_range_zero_prefix() {
        let ip = Ipv4Addr::new(10, 20, 30, 40);
        let prefix = 0;
        let range = cidr_range(ip, prefix).unwrap();

        let expected_start = Ipv4Addr::new(0, 0, 0, 0);
        let expected_end = Ipv4Addr::new(255, 255, 255, 255);

        assert_eq!(range.start_addr, expected_start);
        assert_eq!(range.end_addr, expected_end);
    }

    #[test]
    fn test_cidr_range_32_prefix() {
        let ip = Ipv4Addr::new(172, 16, 0, 1);
        let prefix = 32;
        let range = cidr_range(ip, prefix).unwrap();
        
        assert_eq!(range.start_addr, ip);
        assert_eq!(range.end_addr, ip);
    }

    #[test]
    fn test_cidr_range_invalid_prefix() {
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        let prefix = 33;
        let result = cidr_range(ip, prefix);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Invalid prefix: 33 > 32");
    }

    #[test]
    fn test_from_cidr_str() {
        let cidr = "10.0.0.0/8";
        let range = _from_cidr_str(cidr).unwrap();
        
        let expected_start = Ipv4Addr::new(10, 0, 0, 0);
        let expected_end = Ipv4Addr::new(10, 255, 255, 255);
        
        assert_eq!(range, Ipv4Range::new(expected_start, expected_end));
    }

    #[test]
    fn test_from_cidr_str_invalid() {
        assert!(_from_cidr_str("10.0.0.0/33").is_err());
        assert!(_from_cidr_str("10.0.0.0").is_err());
        assert!(_from_cidr_str("not-an-ip/24").is_err());
        assert!(_from_cidr_str("256.0.0.1/24").is_err());
    }

    #[test]
    fn test_from_ipv4_net() {
        let cidr = "192.168.5.0/24";
        let net = Ipv4Network::from_str(cidr).ok();
        let range = from_ipv4_net(net).unwrap();

        let expected = _from_cidr_str(cidr).unwrap();
        assert_eq!(range, expected);
        
        assert!(from_ipv4_net(None).is_none());
    }

    #[test]
    fn in_optional_range_test() {
        let start = Ipv4Addr::new(10, 0, 0, 10);
        let end = Ipv4Addr::new(10, 0, 0, 20);
        let range = Ipv4Range::new(start, end);
        
        let in_addr = Ipv4Addr::new(10, 0, 0, 15);
        let out_addr = Ipv4Addr::new(10, 0, 0, 5);
        
        let some_range = Some(range);
        let none_range: Option<Ipv4Range> = None;
        
        // `Some` range, address is in
        assert!(in_optional_range(&in_addr, &some_range));
        // `Some` range, address is out
        assert!(!in_optional_range(&out_addr, &some_range));
        
        // `None` range, address is in
        assert!(in_optional_range(&in_addr, &none_range));
        // `None` range, address is out
        assert!(in_optional_range(&out_addr, &none_range));
    }
}