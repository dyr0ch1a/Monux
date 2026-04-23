use core::fsstorage::list;

use crate::commands::context::CommandContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let config = ctx.load_config()?;

    let notes = list::Notes::new(config.notes_dir);

    let list = notes.list()?;

    for note in list {
        println!("{}", note.display());
    }

    Ok(())
}
