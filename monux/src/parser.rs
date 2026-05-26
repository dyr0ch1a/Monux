use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "monux")]
#[command(
    about = "CLI for managing markdown notes in Monux storage",
    long_about = "Monux CLI: initialize storage, create/find/list/sync
notes, and open the line editor."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize Monux config and notes directory
    Init,
    /// Print Monux version
    Version,
    /// Create a new note by name (or path) with optional tags and directory)
    New {
        /// Note name or path
        name: Option<String>,
        /// Tags separated by spaces and/or commas
        #[arg(short, long)]
        tags: Option<String>,
        /// Target directory relative to notes root (example:project/rust)
        #[arg(long)]
        dir: Option<String>,
    },
    /// List notes, optionally filtered by directory prefix
    List {
        /// Directory prefix relative to notes root
        #[arg(long)]
        dir: Option<String>,
        /// List directories instead of notes
        #[arg(long = "dirs")]
        dirs: bool,
        /// Include tags in list output
        #[arg(long = "tags")]
        tags: bool,
        /// Show full note path
        #[arg(long = "path")]
        path: bool,
    },
    /// Find notes by title/path, optionally by tag/content/directory
    Find {
        /// Search query for path/title (and content with --content)
        query: String,
        /// Filter by tag(s), separated by spaces and/or commas
        #[arg(short = 't', long = "tags")]
        tags: Option<String>,
        /// Include full-text search in markdown body
        #[arg(short = 'c', long = "content")]
        content: bool,
        /// Restrict search to directory prefix
        #[arg(long)]
        dir: Option<String>,
        /// Show full note path
        #[arg(long = "path")]
        path: bool,
    },
    /// Delete notes by query (asks confirmation unless --yes)
    Delete {
        /// Query used to find note(s) for deletion
        query: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Rename note path/title
    Rename {
        /// Existing note path
        old: String,
        /// New note path or title
        new: String,
    },
    /// Rename directory path (and all nested note paths)
    RenameDir {
        /// Existing directory path
        old: String,
        /// New directory path
        new: String,
    },
    /// Sync note index with files in notes directory
    Sync {
        /// Restrict sync to directory prefix
        #[arg(long)]
        dir: Option<String>,
    },
    /// Open built-in line editor (optionally with a note path)
    Edit { path: Option<String> },
    /// Launch terminal UI (monux_tui binary)
    Tui,
}
