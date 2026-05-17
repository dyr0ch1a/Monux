use std::io::ErrorKind;

use core::index::{note_path, note_slug_with_dir};

use crate::commands::context::CommandContext;

pub fn run(query: String, tag: Option<String>, content: bool, dir: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;

    let mut found = index.find(&query)?;
    if content {
        let q = query.to_lowercase();
        let content_matches = index
            .list()?
            .into_iter()
            .filter_map(|note| {
                let path = note_path(&config.notes_dir, &note.slug);
                let body = std::fs::read_to_string(path).ok()?;
                if body.to_lowercase().contains(&q) {
                    Some(note)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for note in content_matches {
            if !found.iter().any(|n| n.id == note.id) {
                found.push(note);
            }
        }
    }

    if let Some(prefix_raw) = dir {
        let prefix = note_slug_with_dir(None, &prefix_raw);
        if !prefix.is_empty() {
            found.retain(|n| n.slug.starts_with(&prefix));
        }
    }

    if let Some(tag) = tag {
        let by_tag = index.find_by_tag(&tag)?;
        let by_tag_ids: std::collections::HashSet<u64> = by_tag.into_iter().map(|n| n.id).collect();
        found.retain(|n| by_tag_ids.contains(&n.id));
    }
    if found.is_empty() {
        println!("Nothing found");
        return Ok(());
    }

    for note in found {
        let tags = index.list_tags(note.id)?;
        let path = note_path(&config.notes_dir, &note.slug);
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
            Err(err) => return Err(err.into()),
        };

        if tags.is_empty() {
            println!("{}\t{}", note.id, note.slug);
        } else {
            println!("{}\t{}\t#{}", note.id, note.slug, tags.join(" #"));
        }
        if content.trim().is_empty() {
            println!("[empty]");
        } else {
            print!("{content}");
            if !content.ends_with('\n') {
                println!();
            }
        }
        println!();
    }

    Ok(())
}
