use std::collections::HashSet;

use core::fsstorage::list::Notes;
use core::index::normalize_slug;

use crate::commands::context::CommandContext;

pub fn run(dir: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    let listed = Notes::new(config.notes_dir.clone()).list()?;
    let mut file_slugs = HashSet::new();
    let mut created = 0usize;
    let mut skipped = 0usize;

    for file in listed {
        let Ok(rel) = file.strip_prefix(&config.notes_dir) else {
            skipped += 1;
            continue;
        };

        let rel_no_ext = rel.with_extension("");
        let rel_str = rel_no_ext.to_string_lossy().replace('\\', "/");
        let slug = normalize_slug(&rel_str);
        if slug.is_empty() {
            skipped += 1;
            continue;
        }

        if let Some(dir_filter) = dir.as_deref() {
            let prefix = normalize_slug(dir_filter);
            if !prefix.is_empty() && !slug.starts_with(&prefix) {
                continue;
            }
        }

        file_slugs.insert(slug.clone());

        if index.get_by_slug(&slug)?.is_none() {
            index.create_note(&slug)?;
            created += 1;
        }
    }

    let notes = index.list()?;
    let mut removed = 0usize;
    for note in notes {
        if let Some(dir_filter) = dir.as_deref() {
            let prefix = normalize_slug(dir_filter);
            if !prefix.is_empty() && !note.slug.starts_with(&prefix) {
                continue;
            }
        }

        if !file_slugs.contains(&note.slug) {
            index.delete(note.id)?;
            removed += 1;
        }
    }

    println!("Sync done\tcreated={created}\tremoved={removed}\tskipped={skipped}");
    Ok(())
}
