use std::io::{self, Write};

use core::editor::{Editor, Event, FsStorage};
use core::index::{normalize_slug, note_path};

use crate::commands::context::CommandContext;

pub fn run(slug: Option<String>) -> anyhow::Result<()> {
    let mut editor = Editor::new(FsStorage);

    if let Some(slug) = slug {
        let ctx = CommandContext::new()?;
        let config = ctx.load_config()?;
        let index = ctx.open_note_index()?;

        let normalized = normalize_slug(&slug);
        let note = index
            .get_by_slug(&normalized)?
            .ok_or_else(|| anyhow::anyhow!("note '{}' not found in index", normalized))?;

        let file_path = note_path(&config.notes_dir, &note.slug);
        if !file_path.exists() {
            std::fs::File::create(&file_path)?;
        }

        let outcome = editor.execute(&format!("e {}", file_path.display()))?;
        print_events(&outcome.events);
        println!("editing\t{}\t{}", note.id, note.slug);
    }

    let stdin = io::stdin();
    loop {
        if editor.is_in_input_mode() {
            print!("> ");
        } else {
            print!(": ");
        }
        io::stdout().flush()?;

        let mut line = String::new();
        let read = stdin.read_line(&mut line)?;
        if read == 0 {
            break;
        }

        let line = line.trim_end_matches(['\n', '\r']);
        match editor.execute(line) {
            Ok(outcome) => {
                print_events(&outcome.events);
                if outcome.should_quit {
                    break;
                }
            }
            Err(err) => {
                eprintln!("? {err}");
            }
        }
    }

    Ok(())
}

fn print_events(events: &[Event]) {
    for event in events {
        match event {
            Event::Line(line) => println!("{line}"),
            Event::Message(msg) => println!("{msg}"),
        }
    }
}
