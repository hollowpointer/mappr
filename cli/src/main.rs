mod commands;
mod terminal;

use commands::{CommandLine, Commands, discover::discover, info::info, listen::listen, scan::scan};

use mappr_common::{config::Config, error, network::target};

use crate::terminal::{print, spinner};

#[tokio::main]
async fn main() {
    let commands = CommandLine::parse_args();
    let q_lvl: u8 = commands.quiet;

    spinner::init_logging(commands.verbosity);
    print::banner(commands.no_banner, q_lvl);

    if let Err(e) = run(commands).await {
        error!("Critical failure: {e}");
        print::end_of_program();
        std::process::exit(1)
    }

    if q_lvl == 0 {
        print::end_of_program();
    }
}

async fn run(commands: CommandLine) -> anyhow::Result<()> {
    let cfg = Config {
        no_banner: commands.no_banner,
        no_dns: commands.no_dns,
        redact: commands.redact,
        quiet: commands.quiet,
        disable_input: false,
    };

    match commands.command {
        Commands::Info => {
            print::header("about the tool", cfg.quiet);
            info(&cfg)
        }
        Commands::Listen => {
            print::header("starting listener", cfg.quiet);
            listen()
        }
        Commands::Discover { targets } => {
            print::header("performing host discovery", cfg.quiet);
            let ips = target::to_collection(&targets)?;
            discover(ips, &cfg).await
        }
        Commands::Scan { targets } => {
            print::header("starting scanner", cfg.quiet);
            let ips = target::to_collection(&targets)?;
            scan(ips, &cfg)
        }
    }
}
