use core::context::StorageContext;
use core::fsstorage::list::Notes;

pub fn run(query: String) -> anyhow::Result<()> {
    let ctx = StorageContext::new()?;
    let config = ctx.load_config()?;

    let files = Notes::new(config.notes_dir);

    files.delete(query)?;

    Ok(())
}
