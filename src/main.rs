use cmd::Commands;
use cmd::{discover, listen, info, scan};

mod cmd;

fn main() {
    let commands = cmd::CommandLine::parse_args();
    match commands.command {
        Commands::Info => info::info(),
        Commands::Listen => listen::listen(),
        Commands::Discover { network } => discover::discover(network),
        Commands::Scan { scan_target } => scan::scan(scan_target)
    }
}