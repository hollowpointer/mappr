use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use cmd::Commands;
use cmd::{discover, listen, info, scan};
use crate::utils::print;

mod cmd;
mod net;
mod utils;
mod host;

pub static SPINNER: Lazy<ProgressBar> = Lazy::new(|| {
    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::with_template("{spinner:.blue} {msg}")
        .unwrap()
        .tick_strings(
            &["▁▁▁▁▁",
                "▁▂▂▂▁",
                "▁▄▂▄▁",
                "▂▄▆▄▂",
                "▄▆█▆▄",
                "▂▄▆▄▂",
                "▁▄▂▄▁",
                "▁▂▂▂▁"]
        );
    pb.set_style(style);
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commands = cmd::CommandLine::parse_args();
    print::print_header();
    match commands.command {
        Commands::Info => {
            print::separator("about the tool");
            Ok(info::info()?)
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