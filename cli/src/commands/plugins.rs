use core::index::{normalize_slug, note_path};
use core::plugin::apply_plugins_in_file;

use crate::commands::context::CommandContext;

pub fn run(slug: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    let normalized = normalize_slug(&slug);
    if normalized.is_empty() {
        anyhow::bail!("invalid note slug");
    }

    let note = index
        .get_by_slug(&normalized)?
        .ok_or_else(|| anyhow::anyhow!("note '{}' not found in index", normalized))?;
    let path = note_path(&config.notes_dir, &note.slug);

    let report = apply_plugins_in_file(&path, &config.plugins_dir)?;
    println!("replacements\t{}", report.replacements);
    for err in report.errors {
        eprintln!("plugin error\t{err}");
    }

    Ok(())
}
