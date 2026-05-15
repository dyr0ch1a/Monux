use core::index::parse_tags_input;

use crate::commands::context::CommandContext;

pub fn add(note: String, tags: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;
    let parsed = parse_tags_input(&tags);
    if parsed.is_empty() {
        anyhow::bail!("tags are empty");
    }

    let merged = index.add_tags_to_slug(&note, &parsed)?;
    println!("{}\t#{}", note, merged.join(" #"));
    Ok(())
}

pub fn list(note: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;
    let tags = index.list_tags_by_slug(&note)?;

    if tags.is_empty() {
        println!("{} has no tags", note);
    } else {
        println!("{}\t#{}", note, tags.join(" #"));
    }
    Ok(())
}
