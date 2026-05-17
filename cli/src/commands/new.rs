use core::index::{note_path, note_slug_with_dir, parse_tags_input};

use crate::commands::context::CommandContext;

pub fn run(name: Option<String>, tags: Option<String>, dir: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    std::fs::create_dir_all(&config.notes_dir)?;

    let Some(name) = name else {
        anyhow::bail!("note name is required");
    };

    let slug_input = note_slug_with_dir(dir.as_deref(), &name);
    if slug_input.is_empty() {
        anyhow::bail!("note name is invalid");
    }

    let parsed_tags = tags.map(|v| parse_tags_input(&v)).unwrap_or_default();
    let note = index.create_note_with_tags(&slug_input, &parsed_tags)?;
    let path = note_path(&config.notes_dir, &note.slug);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        std::fs::File::create(path)?;
    }

    if parsed_tags.is_empty() {
        println!("{}", note.slug);
    } else {
        println!("{}\t#{}", note.slug, parsed_tags.join(" #"));
    }
    Ok(())
}
