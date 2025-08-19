pub mod discover;
pub mod listen;
pub mod info;
pub mod scan;

use clap::{Parser, Subcommand, ValueEnum};

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
    Info,
    /// Enumerate a network passively
    Listen,
    /// Discover hosts in a given network
    Discover {
        network: Network,
    },
    /// Scan one or more hosts
    Scan {
        scan_target: String,
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Network {
    LAN,
}

impl CommandLine {
    pub fn parse_args() -> Self { Self::parse() }
}