mod commands;
mod parser;

use clap::Parser;
use commands::{delete, edit, find, init, list, new, rename, sync, tags, version};
use parser::{Cli, Commands, TagsCommands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => version::run(),
        Commands::Init => init::run()?,
        Commands::New { name, tags, dir } => new::run(name, tags, dir)?,
        Commands::List { dir } => list::run(dir)?,
        Commands::Find {
            query,
            tag,
            content,
            dir,
        } => find::run(query, tag, content, dir)?,
        Commands::Delete { query, yes } => delete::run(query, yes)?,
        Commands::Rename { old, new } => rename::run(old, new)?,
        Commands::Sync { dir } => sync::run(dir)?,
        Commands::Edit { path } => edit::run(path)?,
        Commands::Tags { command } => match command {
            TagsCommands::Add { note, tags: note_tags } => tags::add(note, note_tags)?,
            TagsCommands::List { note } => tags::list(note)?,
        },
    }

    Ok(())
}
