//! # IPv4 Range Model
//!
//! Provides utilities for working with continuous ranges of IPv4 addresses.
//!
//! This module is primarily used by the [`crate::domain::models::target::Target`] enum
//! to represent ranges like `192.168.1.1-100` or CIDR `192.168.1.0/24`.

use anyhow;
use std::net::{IpAddr, Ipv4Addr};

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
        let start: u32 = u32::from(self.start_addr);
        let end: u32 = u32::from(self.end_addr);
        (start..=end).map(Ipv4Addr::from)
    }

    pub fn to_iter(&self) -> impl Iterator<Item = IpAddr> {
        self.iter().map(IpAddr::V4)
    }
}

/// Creates a range from an IP and a CIDR prefix (e.g., 192.168.1.0/24).
///
/// Returns the range covering the entire network block.
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
    Ok(Ipv4Range::new(
        Ipv4Addr::from(network),
        Ipv4Addr::from(broadcast),
    ))
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
}
