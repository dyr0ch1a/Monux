use std::io::{self, Write};


use monux_core::editor::{Editor, Event, FsStorage};
use monux_core::index::abs_note_path;


use crate::commands::context::{resolve_note_reference,
CommandContext};


pub fn run(path: Option<String>) -> anyhow::Result<()> {
    let mut editor = Editor::new(FsStorage);


    if let Some(path) = path {
        let ctx = CommandContext::new()?;
        let config = ctx.load_config()?;
        let index = ctx.open_note_index()?;


        let note = resolve_note_reference(&index, &path)?;


        let file_path = abs_note_path(&config.notes_dir, &note.path);
        if !file_path.exists() {
            std::fs::File::create(&file_path)?;
        }


        let outcome = editor.execute(&format!("e {}",
file_path.display()))?;
        print_events(&outcome.events);
        println!("editing\t{}", note.title);
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


