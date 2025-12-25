//! # Mappr Binary
//!
//! The entry point for the `mappr` CLI application.
//!
//! ## Responsibility
//! * Parses command line arguments.
//! * Initializes the terminal interface.
//! * Dispatches commands to the appropriate Inbound Adapter (`src/adapters/inbound/cli`).

mod commands;
mod terminal;

use commands::{Commands, discover, info, listen, scan, CommandLine};
use terminal::print;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commands = CommandLine::parse_args();
    print::initialize();
    match commands.command {
        Commands::Info => {
            print::header("about the tool");
            Ok(info::info()?)
        }
        Commands::Listen => {
            print::header("starting listener");
            Ok(listen::listen())
        }
        Commands::Discover { target } => {
            print::header("getting ready for discovery");
            discover::discover(target).await
        }
        Commands::Scan { target } => {
            print::header("starting scanner");
            Ok(scan::scan(target))
        }
    }
}
