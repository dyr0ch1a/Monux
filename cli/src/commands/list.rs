use core::index::note_slug_with_dir;

use crate::commands::context::CommandContext;

pub fn run(dir: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;

    let dir_prefix = dir
        .as_deref()
        .map(|v| note_slug_with_dir(None, v))
        .filter(|v| !v.is_empty());

    let notes = index.list()?;
    for note in notes {
        if let Some(prefix) = &dir_prefix {
            if !note.slug.starts_with(prefix) {
                continue;
            }
        }
        println!("{}", note.slug);
    }

    Ok(())
}
