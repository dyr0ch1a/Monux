use crossterm::event::KeyCode;

pub enum Action {
    Quit,
    Up,
    Down,
    None,
}

pub fn handle_key(key: KeyCode) -> Action {
    match key {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') => Action::Down,
        KeyCode::Char('k') => Action::Up,
        _ => Action::None,
    }
}
