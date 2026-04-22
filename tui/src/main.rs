use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod input;
mod ui;

use app::App;
use input::{Action, handle_key};

fn main() -> Result<(), std::io::Error> {
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        should_quit: false,
        selected: 0,
        items: vec!["Item 1".into(), "Item 2".into(), "Item 3".into()],
    };
    execute!(terminal.backend_mut(), EnterAlternateScreen);
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match handle_key(key.code) {
                Action::Quit => break,
                Action::Down => app.selected += 1,
                Action::Up => app.selected = app.selected.saturating_sub(1),
                Action::None => {}
            }
        }
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen);
    disable_raw_mode()?;
    Ok(())
}
