use cmd::Commands;
use pnet::datalink;

mod cmd;

fn main() {
    let commands = cmd::CommandLine::parse_args();
    match commands.command {
        Commands::Info => println!("{:?}", datalink::interfaces()),
        Commands::Listen => println!("Listening for devices..."),
        Commands::Discover { network } => println!("Discovering {:?}...", network),
        Commands::Scan { scan_target } => println!("Scanning {}...", scan_target)
    }
}