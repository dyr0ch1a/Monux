mod commands;
mod parser;

use clap::Parser;
use commands::{delete, edit, find, init, list, new, tags, version};
use parser::{Cli, Commands, TagsCommands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => version::run(),
        Commands::Init => init::run()?,
        Commands::New { name, tags } => new::run(name, tags)?,
        Commands::List => list::run()?,
        Commands::Find { query, tag } => find::run(query, tag)?,
        Commands::Delete { query } => delete::run(query)?,
        Commands::Edit { path } => edit::run(path)?,
        Commands::Tags { command } => match command {
            TagsCommands::Add { note, tags: note_tags } => tags::add(note, note_tags)?,
            TagsCommands::List { note } => tags::list(note)?,
        },
    }

    Ok(())
}
