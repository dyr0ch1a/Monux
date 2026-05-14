use crate::commands::context::CommandContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;

    let notes = index.list()?;
    for note in notes {
        println!("{}", note.title);
    }

    Ok(())
}
