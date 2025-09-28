use std::time::Duration;
use indicatif::ProgressBar;
use once_cell::sync::Lazy;
use cmd::Commands;
use cmd::{discover, listen, info, scan};

mod cmd;
mod net;
mod print;
mod host;

pub static SPINNER: Lazy<ProgressBar> = Lazy::new(|| {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commands = cmd::CommandLine::parse_args();
    print::print_header();
    match commands.command {
        Commands::Info => {
            print::separator("sending information");
            Ok(info::info())
        },
        Commands::Listen => {
            print::separator("starting listener");
            Ok(listen::listen())
        },
        Commands::Discover { target } => {
            print::separator("getting ready for discovery");
            discover::discover(target).await
        },
        Commands::Scan { target } => {
            print::separator("starting scanner");
            Ok(scan::scan(target))
        }
    }
}