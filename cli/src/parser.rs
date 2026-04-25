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
    New { name: Option<String> },
    List,
    Find { query: String },
    Delete { query: String },
    Edit { path: Option<String> },
}
