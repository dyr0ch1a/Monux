mod commands;
mod parser;


use clap::Parser;
use commands::{delete, edit, find, init, list, new, rename,
rename_dir, sync, tui, version};
use parser::{Cli, Commands};


fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();


    match cli.command {
        Commands::Version => version::run(),
        Commands::Init => init::run()?,
        Commands::New { name, tags, dir } => new::run(name, tags,
dir)?,
        Commands::List {
            dir,
            dirs,
            tags,
            path,
        } => list::run(dir, dirs, tags, path)?,
        Commands::Find {
            query,
            tags,
            content,
            dir,
            path,
        } => find::run(query, tags, content, dir, path)?,
        Commands::Delete { query, yes } => delete::run(query, yes)?,
        Commands::Rename { old, new } => rename::run(old, new)?,
        Commands::RenameDir { old, new } => rename_dir::run(old,
new)?,
        Commands::Sync { dir } => sync::run(dir)?,
        Commands::Edit { path } => edit::run(path)?,
        Commands::Tui => tui::run()?,
    }


    Ok(())
}

