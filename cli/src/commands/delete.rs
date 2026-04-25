use std::io::ErrorKind;

use core::index::note_path;

use crate::commands::context::CommandContext;

pub fn run(query: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    let found = index.find(&query)?;
    if found.is_empty() {
        println!("Nothing found");
        return Ok(());
    }

    for note in found {
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
