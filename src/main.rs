use cmd::Commands;
use cmd::{discover, listen, info, scan};

mod cmd;
mod net;

fn main() -> anyhow::Result<()> {
    let commands = cmd::CommandLine::parse_args();
    match commands.command {
        Commands::Info => Ok(info::info()),
        Commands::Listen => Ok(listen::listen()),
        Commands::Discover { target } => discover::discover(target),
        Commands::Scan { target } => Ok(scan::scan(target))
    }
}