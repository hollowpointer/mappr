pub mod discover;
pub mod listen;
pub mod info;
pub mod scan;

use std::fmt::{write, Display, Formatter};
use std::net::Ipv4Addr;
use std::str::FromStr;
use clap::{Parser, Subcommand};

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
    CIDR { cidr: String },
    Host { addr: Ipv4Addr },
    Range { start: Ipv4Addr, end: Ipv4Addr },
    VPN,
}

impl CommandLine {
    pub fn parse_args() -> Self { Self::parse() }
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();

        if lower == "lan" { return Ok(Target::LAN); }
        if lower == "vpn" { return Ok(Target::VPN); }

        // host: 192.168.1.10
        if let Ok(ip) = s.parse::<Ipv4Addr>() {
            return Ok(Target::Host { addr: ip });
        }

        // range: 192.168.1.10-192.168.1.50
        if let Some((a, b)) = s.split_once('-') {
            let start = a.parse::<Ipv4Addr>().map_err(|e| e.to_string())?;
            let end   = b.parse::<Ipv4Addr>().map_err(|e| e.to_string())?;
            return Ok(Target::Range { start, end });
        }

        // cidr: 10.0.0.0/24  (basic check)
        if let Some((_, pfx)) = s.split_once('/') {
            pfx.parse::<u8>().map_err(|e| e.to_string())?;
            return Ok(Target::CIDR { cidr: s.to_string() });
        }

        Err(format!("invalid target: {s}"))
    }
}