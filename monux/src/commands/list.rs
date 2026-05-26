use monux_core::index::{normalize_dir_path, path_in_dir, path_key};


use crate::commands::context::CommandContext;


pub fn run(
    dir: Option<String>,
    show_dirs: bool,
    show_tags: bool,
    show_path: bool,
) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;
    let storage = ctx.open_note_storage()?;


    let dir_filter = dir.as_deref();
    if show_dirs {
        let notes = index.list()?;
        let mut dirs = std::collections::BTreeSet::new();
        for note in notes {
            let mut current = note.path.parent();
            while let Some(parent) = current {
                let normalized =
normalize_dir_path(&path_key(parent));
                if !normalized.is_empty() {
                    dirs.insert(normalized);
                }
                current = parent.parent();
            }
        }
        let filter =
dir_filter.map(normalize_dir_path).unwrap_or_default();
        for dir in dirs {
            if !filter.is_empty() && dir != filter &&
!dir.starts_with(&(filter.clone() + "/")) {
                continue;
            }
            println!("{dir}/");
        }
        return Ok(());
    }


    let notes = index.list()?;
    for note in notes {
        if let Some(filter) = dir_filter
            && !path_in_dir(&note.path, filter)
        {
            continue;
        }
        if show_tags {
            let tags = storage.read_tags(&note.path)?;
            if tags.is_empty() {
                if show_path {
                    println!("{}\t{}", note.title,
note.display_path());
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
        } else if show_path {
            println!("{}\t{}", note.title, note.display_path());
        } else {
            println!("{}", note.title);
        }
    }


    Ok(())
}

