use core::index::note_path;

use crate::commands::context::CommandContext;

pub fn run(old: String, new: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    let old_note = index
        .get_by_slug(&old)?
        .ok_or_else(|| anyhow::anyhow!("note '{}' not found", old.trim()))?;
    let renamed = index.rename(&old, &new)?;
    let old_path = note_path(&config.notes_dir, &old_note.slug);
    let new_path = note_path(&config.notes_dir, &renamed.slug);

    if old_path != new_path && old_path.exists() {
        std::fs::rename(&old_path, &new_path)?;
    } else if !new_path.exists() {
        std::fs::File::create(&new_path)?;
    }

    println!("Renamed\t{}\t{}", renamed.id, renamed.slug);
    Ok(())
}
