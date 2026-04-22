mod commands;
mod parser;

use clap::Parser;
use commands::{init, new, version};
use parser::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => version::run(),
        Commands::Init => init::run()?,
        Commands::New { name } => new::run(name)?,
    }

    Ok(())
}
