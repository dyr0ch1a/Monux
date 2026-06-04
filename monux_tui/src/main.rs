use crossterm::{
    cursor::SetCursorStyle,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod markdown;
mod ui;

use app::App;

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let run_result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    run_result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    let mut last_cursor_style: Option<&'static str> = None;
    let mut last_motion_key: Option<(KeyCode, crossterm::event::KeyModifiers, std::time::Instant)> =
        None;

    while !app.should_quit {
        app.poll_filesystem_updates()?;
        app.on_tick();
        #[cfg(not(windows))]
        {
            let cursor_style = app.cursor_style_name();
            if last_cursor_style != Some(cursor_style) {
                match cursor_style {
                    "bar" => execute!(terminal.backend_mut(), SetCursorStyle::SteadyBar)?,
                    "underscore" => {
                        execute!(terminal.backend_mut(), SetCursorStyle::SteadyUnderScore)?
                    }
                    _ => execute!(terminal.backend_mut(), SetCursorStyle::SteadyBlock)?,
                }
                last_cursor_style = Some(cursor_style);
            }
        }
        #[cfg(windows)]
        {
            let _ = &mut last_cursor_style;
        }

        terminal.draw(|f| ui::draw(f, app))?;
        app.load_notes_if_needed()?;

        if !event::poll(std::time::Duration::from_millis(150))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if !matches!(key.kind, KeyEventKind::Press) {
                continue;
            }
            if should_throttle_motion_key(&key, &mut last_motion_key) {
                continue;
            }
            app.on_key(key)?;
        }
    }

    Ok(())
}

fn should_throttle_motion_key(
    key: &KeyEvent,
    last_motion_key: &mut Option<(KeyCode, crossterm::event::KeyModifiers, std::time::Instant)>,
) -> bool {
    let is_motion = matches!(
        key.code,
        KeyCode::Char('h')
            | KeyCode::Char('j')
            | KeyCode::Char('k')
            | KeyCode::Char('l')
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
    );
    if !is_motion {
        return false;
    }

    let now = std::time::Instant::now();
    let threshold = std::time::Duration::from_millis(28);

    if let Some((last_code, last_mods, last_at)) = last_motion_key.as_ref()
        && *last_code == key.code
        && *last_mods == key.modifiers
        && now.duration_since(*last_at) < threshold
    {
        return true;
    }

    *last_motion_key = Some((key.code, key.modifiers, now));
    false
}
