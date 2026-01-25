pub mod discover;
pub mod info;
pub mod listen;
pub mod scan;

use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mappr")]
#[command(about = "A modern network mapper.")]
pub struct CommandLine {
    #[command(subcommand)]
    pub command: Commands,

    /// Keep logs and colors but hide the ASCII art
    #[arg(long = "no-banner", global = true)]
    pub no_banner: bool,

    /// Disables sending of DNS packets
    #[arg(short = 'n', long = "no-dns", global = true)]
    pub no_dns: bool,

    /// Reduce UI visual density (-q: reduce styling, -qq: raw IPs)
    #[arg(short = 'q', long = "quiet", action = ArgAction::Count, global = true)]
    pub quiet: u8,

    /// Redact sensitive info (IPv6 suffixes, MAC addresses etc.)
    #[arg(long = "redact", global = true)]
    pub redact: bool,

    /// Increase logging detail (-v: debug logs, -vv: full packets)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    pub verbosity: u8,
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
        #[arg(value_name = "TARGETS", num_args(1..))]
        targets: Vec<String> 
    },
    
    /// Scan one or more hosts
    #[command(alias = "s")]
    Scan { 
        #[arg(value_name = "TARGETS", num_args(1..))]
        targets: Vec<String> 
    },
}

impl CommandLine {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}