pub mod discover;
pub mod info;
pub mod listen;
pub mod scan;

use clap::{ArgAction, Parser, Subcommand};
use mappr_common::network::target::Target;


#[derive(Parser)]
#[command(name = "mappr")]
#[command(about = "A modern network mapper.")]
pub struct CommandLine {
    #[command(subcommand)]
    pub command: Commands,

    /// Disables sending of DNS packets
    #[arg(short = 'n', long = "no-dns", global = true)]
    pub no_dns: bool,

    /// Verbosity level (-v, -vv, -vvv)
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
    Discover { target: Target },
    /// Scan one or more hosts
    #[command(alias = "s")]
    Scan { target: Target },
}

impl CommandLine {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

