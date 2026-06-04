use monux_core::index::path_in_dir;

use crate::commands::context::CommandContext;

pub fn run(dir: Option<String>) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;
    let storage = ctx.open_note_storage()?;
    let notes = storage.list_note_paths()?;
    for rel in &notes {
        index.reindex_note(rel)?;
    }
    let scanned = notes.len();

    let removed = index.prune_orphan_tags()?;

    if let Some(dir_filter) = dir.as_deref() {
        let count = index
            .list()?
            .into_iter()
            .filter(|note| path_in_dir(&note.path, dir_filter))
            .count();
        println!(
            "Sync
done\tnotes={count}\tscanned={scanned}\torphan_tags_removed={removed}"
        );
    } else {
        let count = index.list()?.len();
        println!(
            "Sync
done\tnotes={count}\tscanned={scanned}\torphan_tags_removed={removed}"
        );
    }

    Ok(())
}
