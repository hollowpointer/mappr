use std::net::Ipv4Addr;
use pnet::ipnetwork::Ipv4Network;

#[derive(Clone, Debug)]
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
}

#[derive(Clone, Debug)]
pub struct IpRange {
    front: u32,
    back: u32,
}

pub fn ip_iter(ipv4_range: &Ipv4Range) -> IpRange {
    IpRange::new(ipv4_range.start_addr, ipv4_range.end_addr)
}

pub fn from_ipv4_net(ipv4_net: Option<Ipv4Network>) -> Option<Ipv4Range> {
    if let Some(ipv4_net) = ipv4_net {
        Some(cidr_range(ipv4_net.ip(), ipv4_net.prefix()))
    } else { None }
}

pub fn cidr_range(ip: Ipv4Addr, prefix: u8) -> Ipv4Range {
    if prefix > 32 { panic!("Not a valid prefix address"); }
    let ip_u32 = u32::from(ip);
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };

    let network = ip_u32 & mask;
    let broadcast = network | !mask;

    Ipv4Range::from_tuple((Ipv4Addr::from(network), Ipv4Addr::from(broadcast)))
}

pub fn in_range(ipv4_addr: &Ipv4Addr, ipv4_range: &Ipv4Range) -> bool {
    (ipv4_range.start_addr..=ipv4_range.end_addr).contains(ipv4_addr)
}

pub fn in_range_optional_range(addr: &Ipv4Addr, range: &Option<Ipv4Range>) -> bool {
    range
    .as_ref()
    .map_or(true, |ipv4_range: &Ipv4Range| in_range(addr, ipv4_range))
}

// fn cidr_str_to_range(cidr: &str) -> Result<(Ipv4Addr, Ipv4Addr)> {
//     let (ip_str, prefix_str) = cidr
//         .split_once('/')
//         .context("CIDR must contain '/'")?;

//     let ip: Ipv4Addr = ip_str
//         .parse()
//         .context("Invalid IPv4 address")?;

//     let prefix: u8 = prefix_str
//         .parse()
//         .context("Invalid prefix")?;

//     Ok(cidr_range(ip, prefix)?)
// }

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

    // #[test]
    // fn cidr_str_to_range_parses_and_computes() {
    //     let (start, end) = cidr_str_to_range("172.16.5.10/20").unwrap();
    //     assert_eq!(start, Ipv4Addr::new(172, 16, 0, 0));
    //     assert_eq!(end,   Ipv4Addr::new(172, 16, 15, 255));
    // }

    // #[test]
    // fn cidr_str_to_range_rejects_bad_ip() {
    //     let res = cidr_str_to_range("999.1.2.3/24");
    //     assert!(res.is_err());
    // }

    // #[test]
    // fn cidr_str_to_range_rejects_bad_prefix() {
    //     let res = cidr_str_to_range("192.168.0.1/33");
    //     assert!(res.is_err());
    // }

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