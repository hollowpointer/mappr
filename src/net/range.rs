use std::net::Ipv4Addr;
use anyhow::{anyhow, bail, Context, Result};
use pnet::datalink::NetworkInterface;
use crate::cmd::Target;
use crate::net::datalink::interface;

#[derive(Clone)]
pub struct Ipv4Range {
    pub start_addr: Ipv4Addr,
    pub end_addr: Ipv4Addr
}

impl Ipv4Range {
    pub fn _new(start_addr: Ipv4Addr, end_addr: Ipv4Addr) -> Self { Self { start_addr, end_addr } }
    pub fn from_tuple(range: (Ipv4Addr, Ipv4Addr)) -> Self {
        Self {
            start_addr: range.0,
            end_addr: range.1
        }
    }
    pub fn contains(&self, ip: &Ipv4Addr) -> bool {
        self.start_addr <= *ip && *ip <= self.end_addr
    }
}

#[derive(Clone, Debug)]
pub struct IpRange {
    front: u32,
    back: u32,
}

pub fn ip_iter(ipv4range: &Ipv4Range) -> IpRange {
    IpRange::new(ipv4range.start_addr, ipv4range.end_addr)
}

pub fn ip_range(target: Target, intf: &NetworkInterface) -> Result<(Ipv4Addr, Ipv4Addr)> {
    match target {
        Target::LAN | Target::VPN => interface_range_v4(intf),
        Target::CIDR { cidr } => cidr_str_to_range(&cidr),
        Target::Host { addr } => Ok((addr, addr)),
        Target::Range { start, end } => Ok((start, end)),
    }
}

pub fn interface_range_v4(intf: &NetworkInterface) -> Result<(Ipv4Addr, Ipv4Addr)> {
    let ip = interface::get_ipv4(intf)
        .map_err(|_| anyhow!("Failed to get IPv4 from interface"))?;

    let prefix = interface::get_prefix(intf)
        .map_err(|_| anyhow!("Failed to get prefix from interface"))?;

    cidr_range(ip, prefix)
}

fn cidr_range(ip: Ipv4Addr, prefix: u8) -> Result<(Ipv4Addr, Ipv4Addr)> {
    if prefix > 32 { bail!("Not a valid prefix address"); }
    let ip_u32 = u32::from(ip);
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };

    let network = ip_u32 & mask;
    let broadcast = network | !mask;

    Ok((Ipv4Addr::from(network+1) , Ipv4Addr::from(broadcast-1)))
}

fn cidr_str_to_range(cidr: &str) -> Result<(Ipv4Addr, Ipv4Addr)> {
    let (ip_str, prefix_str) = cidr
        .split_once('/')
        .context("CIDR must contain '/'")?;

    let ip: Ipv4Addr = ip_str
        .parse()
        .context("Invalid IPv4 address")?;

    let prefix: u8 = prefix_str
        .parse()
        .context("Invalid prefix")?;

    Ok(cidr_range(ip, prefix)?)
}

impl IpRange {
    pub fn new(start: Ipv4Addr, end: Ipv4Addr) -> Self {
        Self { front: u32::from(start), back: u32::from(end) }
    }
}

impl Iterator for IpRange {
    type Item = Ipv4Addr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front > self.back {
            return None;
        }
        let ip = Ipv4Addr::from(self.front);
        self.front = self.front.saturating_add(1);
        Some(ip)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = if self.front > self.back {
            0
        } else {
            // safe for IPv4 ranges (max 2^32 items, but we only expose usize)
            (self.back - self.front + 1) as usize
        };
        (len, Option::from(len))
    }
}

impl DoubleEndedIterator for IpRange {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front > self.back {
            return None;
        }
        let ip = Ipv4Addr::from(self.back);
        self.back = self.back.saturating_sub(1);
        Some(ip)
    }
}

impl ExactSizeIterator for IpRange {}



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
    fn cidr_range_basic_24() {
        let ip = Ipv4Addr::new(192, 168, 1, 42);
        let (start, end) = cidr_range(ip, 24).unwrap();
        assert_eq!(start, Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(end,   Ipv4Addr::new(192, 168, 1, 255));
    }

    #[test]
    fn cidr_range_prefix_0() {
        let ip = Ipv4Addr::new(10, 20, 30, 40);
        let (start, end) = cidr_range(ip, 0).unwrap();
        assert_eq!(start, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(end,   Ipv4Addr::new(255, 255, 255, 255));
    }

    #[test]
    fn cidr_range_prefix_32_single_host() {
        let ip = Ipv4Addr::new(203, 0, 113, 7);
        let (start, end) = cidr_range(ip, 32).unwrap();
        assert_eq!(start, ip);
        assert_eq!(end,   ip);
    }

    #[test]
    fn cidr_str_to_range_parses_and_computes() {
        let (start, end) = cidr_str_to_range("172.16.5.10/20").unwrap();
        assert_eq!(start, Ipv4Addr::new(172, 16, 0, 0));
        assert_eq!(end,   Ipv4Addr::new(172, 16, 15, 255));
    }

    #[test]
    fn cidr_str_to_range_rejects_bad_ip() {
        let res = cidr_str_to_range("999.1.2.3/24");
        assert!(res.is_err());
    }

    #[test]
    fn cidr_str_to_range_rejects_bad_prefix() {
        let res = cidr_str_to_range("192.168.0.1/33");
        assert!(res.is_err());
    }

    #[test]
    fn iter_forward_and_back() {
        let a = Ipv4Addr::new(192,168,1,1);
        let b = Ipv4Addr::new(192,168,1,3);
        let v: Vec<_> = IpRange::new(a, b).collect();
        assert_eq!(v, vec![
            Ipv4Addr::new(192,168,1,1),
            Ipv4Addr::new(192,168,1,2),
            Ipv4Addr::new(192,168,1,3),
        ]);

        let v2: Vec<_> = IpRange::new(a, b).rev().collect();
        assert_eq!(v2, vec![
            Ipv4Addr::new(192,168,1,3),
            Ipv4Addr::new(192,168,1,2),
            Ipv4Addr::new(192,168,1,1),
        ]);
    }

    #[test]
    fn empty_when_start_gt_end() {
        let a = Ipv4Addr::new(10,0,0,2);
        let b = Ipv4Addr::new(10,0,0,1);
        assert_eq!(IpRange::new(a,b).count(), 0);
    }

}