use cmd::Commands;
use cmd::{discover, listen, info, scan};

mod cmd;
mod net;

fn main() {
    let commands = cmd::CommandLine::parse_args();
    match commands.command {
        Commands::Info => info::info(),
        Commands::Listen => listen::listen(),
        Commands::Discover { target } => discover::discover(target),
        Commands::Scan { target } => scan::scan(target)
    }
}