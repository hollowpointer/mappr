use std::net::Ipv4Addr;
use pnet::datalink::NetworkInterface;
use crate::cmd::Target;
use crate::net::interface;

pub fn ip_range(target: Target, intf: &NetworkInterface) -> (Ipv4Addr, Ipv4Addr) {
    match target {
        Target::LAN => { local_range(&intf) },
        Target::CIDR { cidr } =>  cidr_str_to_range(&cidr),
        Target::Host { addr } => (addr, addr),
        Target::Range { start, end } => (start, end),
        Target::VPN => local_range(&intf)
    }
}

fn local_range(intf: &NetworkInterface) -> (Ipv4Addr, Ipv4Addr) {
    if let Ok(ip) = interface::get_ipv4(&intf) {
        if let Ok(prefix) = interface::get_prefix(&intf) {
            cidr_range(ip, prefix)
        } else {
            eprintln!("Failed to get the ip prefix from interface!");
            (Ipv4Addr::new(0,0,0,0), Ipv4Addr::new(0,0,0,0))
        }
    }
    else {
        eprintln!("Failed to get the ipv4 from interface!");
        (Ipv4Addr::new(0,0,0,0), Ipv4Addr::new(0,0,0,0))
    }
}

fn cidr_range(ip: Ipv4Addr, prefix: u8) -> (Ipv4Addr, Ipv4Addr) {
    if prefix > 32 {
        panic!("Not a valid prefix address");
    }
    let ip_u32 = u32::from(ip);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };

    let network = ip_u32 & mask;
    let broadcast = network | !mask;

    (Ipv4Addr::from(network), Ipv4Addr::from(broadcast))
}

fn cidr_str_to_range(cidr: &str) -> (Ipv4Addr, Ipv4Addr) {
    let Some((ip_str, prefix_str)) = cidr.split_once('/') else { todo!() };
    let Ok(ip) = ip_str.parse::<Ipv4Addr>() else {
        panic!("Not a valid IPv4 address");
    };
    let Ok(prefix) = prefix_str.parse::<u8>() else {
        panic!("Not a valid prefix address");
    };

    cidr_range(ip, prefix)
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
    fn cidr_range_basic_24() {
        let ip = Ipv4Addr::new(192, 168, 1, 42);
        let (start, end) = cidr_range(ip, 24);
        assert_eq!(start, Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(end,   Ipv4Addr::new(192, 168, 1, 255));
    }

    #[test]
    fn cidr_range_prefix_0() {
        let ip = Ipv4Addr::new(10, 20, 30, 40);
        let (start, end) = cidr_range(ip, 0);
        assert_eq!(start, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(end,   Ipv4Addr::new(255, 255, 255, 255));
    }

    #[test]
    fn cidr_range_prefix_32_single_host() {
        let ip = Ipv4Addr::new(203, 0, 113, 7);
        let (start, end) = cidr_range(ip, 32);
        assert_eq!(start, ip);
        assert_eq!(end,   ip);
    }

    #[test]
    fn cidr_str_to_range_parses_and_computes() {
        let (start, end) = cidr_str_to_range("172.16.5.10/20");
        assert_eq!(start, Ipv4Addr::new(172, 16, 0, 0));
        assert_eq!(end,   Ipv4Addr::new(172, 16, 15, 255));
    }

    #[test]
    #[should_panic(expected = "Not a valid IPv4 address")]
    fn cidr_str_to_range_rejects_bad_ip() {
        let _ = cidr_str_to_range("999.1.2.3/24");
    }

    #[test]
    #[should_panic(expected = "Not a valid prefix address")]
    fn cidr_str_to_range_rejects_bad_prefix() {
        let _ = cidr_str_to_range("192.168.0.1/33");
    }
}