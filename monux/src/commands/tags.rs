use monux_core::index::{parse_tags_input, resolve_note_path};

use crate::commands::context::CommandContext;

pub fn add(note: String, tags: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;
    let parsed = parse_tags_input(&tags);
    if parsed.is_empty() {
        anyhow::bail!("tags are empty");
    }

    let rel = resolve_note_path(&note);
    if rel.as_os_str().is_empty() {
        anyhow::bail!("note path is invalid");
    }
    let merged = index.add_tags(&rel, &parsed)?;
    println!("{}\t#{}", rel.display(), merged.join(" #"));
    Ok(())
}

pub fn list(note: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;
    let rel = resolve_note_path(&note);
    if rel.as_os_str().is_empty() {
        anyhow::bail!("note path is invalid");
    }
    let tags = index.list_tags(&rel)?;

    if tags.is_empty() {
        println!("{} has no tags", rel.display());
    } else {
        println!("{}\t#{}", rel.display(), tags.join(" #"));
    }
    Ok(())
}
