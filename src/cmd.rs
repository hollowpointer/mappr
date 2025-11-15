pub mod discover;
pub mod listen;
pub mod info;
pub mod scan;

use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use clap::{Parser, Subcommand};

use crate::net::range::{self, Ipv4Range};

#[derive(Parser)]
#[command(name = "mappr")]
#[command(about = "A modern network mapper.")]
pub struct CommandLine {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show networking information about this device
    #[command(alias = "i")]
    Info,
    /// Enumerate a network passively
    #[command(alias = "l")]
    Listen,
    /// Discover hosts in a given network
    #[command(alias = "d")]
    Discover {
        target: Target,
    },
    /// Scan one or more hosts
    #[command(alias = "s")]
    Scan {
        target: Target,
    }
}

#[derive(Clone, Debug)]
pub enum Target {
    LAN,
    Host { dst_addr: IpAddr },
    Range { ipv4_range: Ipv4Range },
    VPN,
}

impl CommandLine {
    pub fn parse_args() -> Self { Self::parse() }
}

impl FromStr for Target {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();

        if let Some(target) = parse_keyword(&lower) {
            return Ok(target);
        }

        if let Some(target) = parse_host(s) {
            return Ok(target);
        }

        if let Some(target) = parse_ip_range(s)? {
            return Ok(target);
        }

        if let Some(target) = parse_cidr_range(s)? {
            return Ok(target);
        }

        Err(format!("invalid target: {s}"))
    }
}


fn parse_keyword(s_lower: &str) -> Option<Target> {
    match s_lower {
        "lan" => Some(Target::LAN),
        "vpn" => Some(Target::VPN),
        _ => None,
    }
}


fn parse_host(s: &str) -> Option<Target> {
    s.parse::<IpAddr>()
        .ok()
        .map(|dst_addr| Target::Host { dst_addr })
}


fn parse_ip_range(s: &str) -> Result<Option<Target>, String> {
    let Some((start_str, end_str)) = s.split_once('-') else {
        return Ok(None);
    };

    let start_addr = start_str
        .parse::<Ipv4Addr>()
        .map_err(|e| format!("Invalid start IP in range '{start_str}': {e}"))?;

    let end_addr = parse_range_end_addr(end_str, &start_addr, s)?;

    let ipv4_range = Ipv4Range::new(start_addr, end_addr);
    Ok(Some(Target::Range { ipv4_range }))
}


fn parse_range_end_addr(
    end_str: &str,
    start_addr: &Ipv4Addr,
    original_s: &str,
) -> Result<Ipv4Addr, String> {
    if let Ok(full_addr) = end_str.parse::<Ipv4Addr>() {
        return Ok(full_addr);
    }

    let mut end_octets = start_addr.octets();
    let partial_octets: Vec<u8> = end_str
        .split('.')
        .map(|octet_str| octet_str.parse::<u8>())
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| format!("Invalid end range '{end_str}': {e}"))?;

    if partial_octets.is_empty() {
        return Err(format!("End range cannot be empty: {original_s}"));
    }
    if partial_octets.len() > 4 {
        return Err(format!("End range has too many octets: {end_str}"));
    }

    let partial_len = partial_octets.len();
    let start_index = 4 - partial_len;
    end_octets[start_index..].copy_from_slice(&partial_octets);

    Ok(Ipv4Addr::from(end_octets))
}


fn parse_cidr_range(s: &str) -> Result<Option<Target>, String> {
    let Some((ip_str, prefix_str)) = s.split_once('/') else {
        return Ok(None);
    };

    let ipv4_addr = ip_str
        .parse::<Ipv4Addr>()
        .map_err(|e| format!("Invalid IP in CIDR '{ip_str}': {e}"))?;

    let prefix = prefix_str
        .parse::<u8>()
        .map_err(|e| format!("Invalid prefix in CIDR '{prefix_str}': {e}"))?;

    let ipv4_range = range::cidr_range(ipv4_addr, prefix)
        .map_err(|e| e.to_string())?;

    Ok(Some(Target::Range { ipv4_range }))
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
    fn test_parse_range_end_addr_helper() {
        let start = Ipv4Addr::new(192, 168, 1, 10);
        let s = "192.168.1.10-255";

        // Test full IP end
        assert_eq!(
            parse_range_end_addr("192.168.1.50", &start, s),
            Ok(Ipv4Addr::new(192, 168, 1, 50))
        );

        // Test partial 1-octet end
        assert_eq!(
            parse_range_end_addr("50", &start, s),
            Ok(Ipv4Addr::new(192, 168, 1, 50))
        );

        // Test partial 2-octet end
        assert_eq!(
            parse_range_end_addr("2.66", &start, s),
            Ok(Ipv4Addr::new(192, 168, 2, 66))
        );

        // Test partial 3-octet end
        assert_eq!(
            parse_range_end_addr("10.2.1", &start, s),
            Ok(Ipv4Addr::new(192, 10, 2, 1))
        );
        
        // Test partial 4-octet end (same as full)
        assert_eq!(
            parse_range_end_addr("10.20.30.40", &start, s),
            Ok(Ipv4Addr::new(10, 20, 30, 40))
        );

        // --- Error Cases ---
        
        // Invalid octet
        let err_s = "192.168.1.10-2.256";
        assert!(parse_range_end_addr("2.256", &start, err_s).is_err());
        
        // Too many octets
        let err_s = "192.168.1.10-1.2.3.4.5";
        assert!(parse_range_end_addr("1.2.3.4.5", &start, err_s).is_err());
        
        // Empty octets
        let err_s = "192.168.1.10-";
        assert!(parse_range_end_addr("", &start, err_s).is_err());
    }

    #[test]
    fn test_from_str_full_parsing() {
        // Test keywords (case-insensitive)
        assert!(matches!(Target::from_str("lan"), Ok(Target::LAN)));
        assert!(matches!(Target::from_str("VPN"), Ok(Target::VPN)));

        // Test host
        assert!(matches!(Target::from_str("1.1.1.1"), Ok(Target::Host { .. })));
        assert!(matches!(Target::from_str("::1"), Ok(Target::Host { .. })));
        
        // Test full range
        assert!(matches!(Target::from_str("10.0.0.1-10.0.0.255"), Ok(Target::Range { .. })));
        
        // Test partial range
        assert!(matches!(Target::from_str("192.168.1.1-255"), Ok(Target::Range { .. })));
        assert!(matches!(Target::from_str("192.168.1.1-2.255"), Ok(Target::Range { .. })));
        
        // Test CIDR
        assert!(matches!(Target::from_str("10.0.0.0/24"), Ok(Target::Range { .. })));

        // Test invalid
        assert!(Target::from_str("not-an-ip").is_err());
        assert!(Target::from_str("10.0.0.1/33").is_err());
        assert!(Target::from_str("10.0.0.256-1.1.1.1").is_err());
    }
}