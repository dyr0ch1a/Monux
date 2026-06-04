use monux_core::fsstorage::storage::tags_frontmatter;
use monux_core::index::{note_path_with_dir, parse_tags_input, path_key};

use crate::commands::context::CommandContext;

pub fn run(name: Option<String>, tags: Option<String>, dir: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;
    let storage = ctx.open_note_storage()?;

    std::fs::create_dir_all(&config.notes_dir)?;

    let Some(name) = name else {
        anyhow::bail!("note name is required");
    };

    let rel = note_path_with_dir(dir.as_deref(), &name);
    if rel.as_os_str().is_empty() {
        anyhow::bail!("note name is invalid");
    }

    let parsed_tags = tags.map(|v| parse_tags_input(&v)).unwrap_or_default();
    let content = tags_frontmatter(&parsed_tags);
    storage.create_note(&rel, &content)?;
    index.reindex_note(&rel)?;

    if parsed_tags.is_empty() {
        println!("{}", path_key(&rel));
    } else {
        println!("{}\t#{}", path_key(&rel), parsed_tags.join(" #"));
    }
    Ok(())
}
