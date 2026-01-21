mod commands;
mod terminal;

use commands::{CommandLine, Commands, discover, info, listen, scan};
use mappr_common::{config::Config, error};
use terminal::print;

use crate::terminal::spinner;

#[tokio::main]
async fn main() {
    let commands = CommandLine::parse_args();

    spinner::init_logging(commands.verbosity);
    print::initialize();

    let cfg = Config {
        no_dns: commands.no_dns,
    };
    
    let result: Result<(), anyhow::Error> = match commands.command {
        Commands::Info => {
            print::header("about the tool");
            info::info()
        }
        Commands::Listen => {
            print::header("starting listener");
            Ok(listen::listen())
        }
        Commands::Discover { target } => {
            print::header("performing host discovery");
            discover::discover(target, &cfg).await
        }
        Commands::Scan { target } => {
            print::header("starting scanner");
            Ok(scan::scan(target))
        }
    };

    if let Err(e) = result {
        error!("Critical failure: {e}");
        print::end_of_program();
    }
}