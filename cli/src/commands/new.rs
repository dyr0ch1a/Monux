use core::index::{note_path, parse_tags_input};

use crate::commands::context::CommandContext;

pub fn run(name: Option<String>, tags: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    std::fs::create_dir_all(&config.notes_dir)?;

    let Some(name) = name else {
        anyhow::bail!("note name is required");
    };

    let parsed_tags = tags.map(|v| parse_tags_input(&v)).unwrap_or_default();
    let note = index.create_note_with_tags(&name, &parsed_tags)?;
    let path = note_path(&config.notes_dir, &note.slug);
    if !path.exists() {
        std::fs::File::create(path)?;
    }

    if parsed_tags.is_empty() {
        println!("{}", note.title);
    } else {
        println!("{}\t#{}", note.title, parsed_tags.join(" #"));
    }
    Ok(())
}
