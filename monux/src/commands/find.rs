use std::io::ErrorKind;

use monux_core::index::{abs_note_path, parse_tags_input, path_in_dir};

use crate::commands::context::CommandContext;

pub fn run(
    query: String,
    tags: Option<String>,
    content: bool,
    dir: Option<String>,
    show_path: bool,
) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;
    let index = ctx.open_note_index()?;
    let storage = ctx.open_note_storage()?;

    let mut found = index.find(&query)?;
    if content {
        let q = query.to_lowercase();
        let content_matches = index
            .list()?
            .into_iter()
            .filter_map(|note| {
                let path = abs_note_path(&config.notes_dir, &note.path);
                let body = std::fs::read_to_string(path).ok()?;
                if body.to_lowercase().contains(&q) {
                    Some(note)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for note in content_matches {
            if !found.iter().any(|n| n.path == note.path) {
                found.push(note);
            }
        }
    }

    if let Some(dir_filter) = dir.as_deref() {
        found.retain(|n| path_in_dir(&n.path, dir_filter));
    }

    if let Some(tags_input) = tags {
        let parsed = parse_tags_input(&tags_input);
        if parsed.is_empty() {
            anyhow::bail!("tags are empty");
        }
        for tag in parsed {
            let by_tag = index.find_by_tag(&tag)?;
            let by_tag_paths: std::collections::HashSet<_> =
                by_tag.iter().map(|n| n.path.clone()).collect();
            found.retain(|n| by_tag_paths.contains(&n.path));
        }
    }
    if found.is_empty() {
        println!("Nothing found");
        return Ok(());
    }

    for note in found {
        let tags = storage.read_tags(&note.path)?;
        let path = abs_note_path(&config.notes_dir, &note.path);
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
            Err(err) => return Err(err.into()),
        };

        if tags.is_empty() {
            if show_path {
                println!("{}\t{}", note.title, note.display_path());
            } else {
                println!("{}", note.title);
            }
        } else if show_path {
            println!(
                "{}\t{}\t#{}",
                note.title,
                note.display_path(),
                tags.join(" #")
            );
        } else {
            println!("{}\t#{}", note.title, tags.join(" #"));
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
