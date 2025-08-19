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

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Target {
    LAN,
}

impl CommandLine {
    pub fn parse_args() -> Self { Self::parse() }
}