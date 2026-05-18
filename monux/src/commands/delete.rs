use std::io::{self, ErrorKind, Write};

use monux_core::index::note_path;

use crate::commands::context::CommandContext;

pub fn run(query: String, yes: bool) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    let found = index.find(&query)?;
    if found.is_empty() {
        println!("Nothing found");
        return Ok(());
    }

    for note in found {
        if !yes {
            print!("Delete '{}'? [y/N]: ", note.slug);
            io::stdout().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let confirmed = matches!(answer.trim(), "y" | "Y" | "yes" | "YES");
            if !confirmed {
                println!("Skipped\t{}\t{}", note.id, note.slug);
                continue;
            }
        }

        let path = note_path(&config.notes_dir, &note.slug);
        match std::fs::remove_file(path) {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        index.delete(note.id)?;
        println!("Deleted\t{}\t{}", note.id, note.slug);
    }

    Ok(())
}
