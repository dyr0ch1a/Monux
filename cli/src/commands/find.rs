use core::context::StorageContext;
use core::fsstorage::list::Notes;

pub fn run(query: String) -> anyhow::Result<()> {
    let ctx = StorageContext::new()?;
    let config = ctx.load_config()?;

    let notes = Notes::new(config.notes_dir);

    let found = notes.find(query)?;

    for path in found {
        println!("{}", path.display());
    }

    Ok(())
}
