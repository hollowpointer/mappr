mod commands;
mod terminal;

use commands::{CommandLine, Commands, discover, info, listen, scan};
use mappr_common::config::Config;
use terminal::print;

use crate::terminal::spinner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commands = CommandLine::parse_args();

    spinner::init_logging();
    print::initialize();

    let cfg = Config {
        no_dns: commands.no_dns,
    };
    
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
            discover::discover(target, &cfg).await
        }
        Commands::Scan { target } => {
            print::header("starting scanner");
            Ok(scan::scan(target))
        }
    }
}