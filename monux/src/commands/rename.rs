use monux_core::index::resolve_note_path;


use crate::commands::context::{resolve_note_reference,
CommandContext};


pub fn run(old: String, new: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;


    let old_note = resolve_note_reference(&index, &old)?;
    let new_rel = resolve_note_path(&new);
    if new_rel.as_os_str().is_empty() {
        anyhow::bail!("new note name cannot be empty");
    }


    if old_note.path == new_rel {
        println!("Renamed\t{}", new_rel.display());
        return Ok(());
    }
    index.move_note(&old_note.path, &new_rel)?;


    println!("Renamed\t{}", new_rel.display());
    Ok(())
}


