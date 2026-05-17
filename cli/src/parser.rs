use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "monux")]
#[command(about = "TUI app Obsidian like")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Init,
    Version,
    New {
        name: Option<String>,
        #[arg(short, long)]
        tags: Option<String>,
        #[arg(long)]
        dir: Option<String>,
    },
    List {
        #[arg(long)]
        dir: Option<String>,
    },
    Find {
        query: String,
        #[arg(short, long)]
        tag: Option<String>,
        #[arg(short = 'c', long = "content")]
        content: bool,
        #[arg(long)]
        dir: Option<String>,
    },
    Delete {
        query: String,
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    Rename {
        old: String,
        new: String,
    },
    Sync {
        #[arg(long)]
        dir: Option<String>,
    },
    Edit { path: Option<String> },
    Tags {
        #[command(subcommand)]
        command: TagsCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum TagsCommands {
    Add {
        note: String,
        tags: String,
    },
    List {
        note: String,
    },
}
