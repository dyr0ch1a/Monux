mod commands;
mod parser;

use clap::Parser;
use commands::{delete, edit, find, init, list, new, plugins, version};
use parser::{Cli, Commands, PluginCommands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => version::run(),
        Commands::Init => init::run()?,
        Commands::New { name } => new::run(name)?,
        Commands::List => list::run()?,
        Commands::Find { query } => find::run(query)?,
        Commands::Delete { query } => delete::run(query)?,
        Commands::Edit { path } => edit::run(path)?,
        Commands::Plugins { command } => match command {
            PluginCommands::Run { slug } => plugins::run(slug)?,
        },
    }

    Ok(())
}
