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
    },
    List,
    Find {
        query: String,
        #[arg(short, long)]
        tag: Option<String>,
    },
    Delete { query: String },
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
