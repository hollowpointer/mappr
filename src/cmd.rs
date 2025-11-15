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
    CIDR { ipv4_range: Ipv4Range },
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

        if lower == "lan" { return Ok(Target::LAN); }
        if lower == "vpn" { return Ok(Target::VPN); }

        // host: 192.168.1.10
        if let Ok(ip) = s.parse::<IpAddr>() {
            return Ok(Target::Host { dst_addr: ip });
        }

        // range: 192.168.1.10-192.168.1.50 or 192.168.0.1-2.66
        if let Some((a, b)) = s.split_once('-') {
            let start_addr: Ipv4Addr = a.parse::<Ipv4Addr>().map_err(|e| e.to_string())?;            
            let end_addr: Ipv4Addr = match b.parse::<Ipv4Addr>() {
                Ok(full_addr) => {
                    full_addr
                }
                Err(_) => {
                    let mut end_octets = start_addr.octets();             
                    let partial_octets: Vec<u8> = b.split('.')
                        .map(|octet_str| octet_str.parse::<u8>())
                        .collect::<Result<Vec<u8>, _>>()
                        .map_err(|e| format!("Invalid end range '{b}': {e}"))?;

                    if partial_octets.is_empty() {
                        return Err(format!("End range cannot be empty: {s}"));
                    }
                    if partial_octets.len() > 4 {
                        return Err(format!("End range has too many octets: {b}"));
                    }
                    let partial_len = partial_octets.len();
                    let start_index = 4 - partial_len;
                    end_octets[start_index..].copy_from_slice(&partial_octets);
                    Ipv4Addr::from(end_octets)
                }
            };
            
            // 3. Create the range (no change here)
            let ipv4_range: Ipv4Range = Ipv4Range::new(start_addr, end_addr);
            return Ok(Target::Range { ipv4_range });
        }

        // cidr: 10.0.0.0/24  (basic check)
        if let Some((ip_str, prefix_str)) = s.split_once('/') {
            let ip_result = ip_str.parse::<Ipv4Addr>();
            let prefix_result = prefix_str.parse::<u8>();
            if let (Ok(ipv4_addr), Ok(prefix)) = (ip_result, prefix_result) {
                let ipv4_range: Ipv4Range = range::cidr_range(ipv4_addr, prefix);
                return Ok(Target::CIDR { ipv4_range });
            }
        }

        Err(format!("invalid target: {s}"))
    }
}