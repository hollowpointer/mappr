mod commands;
mod terminal;

use commands::{
    CommandLine, 
    Commands, 
    discover::discover, 
    info::info, 
    listen::listen,
    scan::scan
};

use mappr_common::{
    config::Config, 
    error, network::target,
};

use crate::terminal::{
    spinner, 
    print
};

#[tokio::main]
async fn main() {
    let commands = CommandLine::parse_args();
    spinner::init_logging(commands.verbosity);
    if !commands.no_banner { print::banner(); };

    if let Err(e) = run(commands).await {
        error!("Critical failure: {e}");
        print::end_of_program();
        std::process::exit(1)
    }

    print::end_of_program();
}

async fn run(commands: CommandLine) -> anyhow::Result<()> {
    let cfg = Config { 
        no_dns: commands.no_dns,
        redact: commands.redact
    };

    match commands.command {
        Commands::Info => {
            print::header("about the tool");
            info()?;
        }
        Commands::Listen => {
            print::header("starting listener");
            listen()?;
        }
        Commands::Discover { targets } => {
            print::header("performing host discovery");
            let ips = target::to_collection(&targets)?;
            discover(ips, &cfg).await?;
        }
        Commands::Scan { targets } => {
            print::header("starting scanner");    
            let ips = target::to_collection(&targets)?;
            scan(ips, &cfg)?;
        }
    }
    
    Ok(())
}