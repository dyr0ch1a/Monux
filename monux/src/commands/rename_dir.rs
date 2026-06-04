use monux_core::index::normalize_dir_path;

use crate::commands::context::CommandContext;

pub fn run(old: String, new: String) -> anyhow::Result<()> {
    let old_dir = normalize_dir_path(&old);
    let new_dir = normalize_dir_path(&new);
    if old_dir.is_empty() || new_dir.is_empty() {
        anyhow::bail!("directory path cannot be empty");
    }
    if old_dir == new_dir {
        println!("directory is already '{new_dir}'");
        return Ok(());
    }
    if new_dir.starts_with(&(old_dir.clone() + "/")) {
        anyhow::bail!("cannot rename directory into its own child");
    }

    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;
    index.rename_dir(&old_dir, &new_dir)?;

    println!("Renamed dir\t{old_dir}\t{new_dir}");
    Ok(())
}
