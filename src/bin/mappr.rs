use mappr::utils::print;
use mappr::cmd::{Commands, discover, info, listen, scan};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commands = mappr::cmd::CommandLine::parse_args();
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
