use std::io::{self, Write};


use crate::commands::context::CommandContext;


pub fn run(query: String, yes: bool) -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    let index = ctx.open_note_index()?;


    let found = index.find(&query)?;
    if found.is_empty() {
        println!("Nothing found");
        return Ok(());
    }


    for note in found {
        if !yes {
            print!("Delete '{}'? [y/N]: ", note.title);
            io::stdout().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let confirmed = matches!(answer.trim(), "y" | "Y" | "yes"
| "YES");
            if !confirmed {
                println!("Skipped\t{}", note.title);
                continue;
            }
        }


        index.delete_note(&note.path)?;
        println!("Deleted\t{}", note.title);
    }


    Ok(())
}


