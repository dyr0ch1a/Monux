use core::index::note_path;

use crate::commands::context::CommandContext;

pub fn run(name: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    std::fs::create_dir_all(&config.notes_dir)?;

    let Some(name) = name else {
        anyhow::bail!("note name is required");
    };

    let note = index.create_note(&name)?;
    let path = note_path(&config.notes_dir, &note.slug);
    if !path.exists() {
        std::fs::File::create(path)?;
    }

    println!("{}\t{}", note.id, note.slug);
    Ok(())
}
