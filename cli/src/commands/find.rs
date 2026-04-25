use crate::commands::context::CommandContext;

pub fn run(query: String) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;

    let found = index.find(&query)?;
    for note in found {
        println!("{}\t{}\t{}", note.id, note.slug, note.title);
    }

    Ok(())
}
