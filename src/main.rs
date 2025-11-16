use crate::utils::print;
use cmd::Commands;
use cmd::{discover, info, listen, scan};
use std::cell::Cell;

mod cmd;
mod host;
mod net;
mod utils;

thread_local! {
    static GLOBAL_KEY_WIDTH: Cell<usize> = Cell::new(0);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commands = cmd::CommandLine::parse_args();
    print::print_banner();
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
