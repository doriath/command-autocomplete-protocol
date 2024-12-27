use casp::carapace::{run_carapace, CarapaceArgs};
use casp::nushell::{run_nushell, NushellArgs};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct AppArgs {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Carapace(CarapaceArgs),
    Nushell(NushellArgs),
}

fn main() -> anyhow::Result<()> {
    let args = AppArgs::parse();

    match args.command {
        Command::Carapace(args) => run_carapace(args),
        Command::Nushell(args) => run_nushell(args),
    }
}
