use ratatui::{
    Frame,
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.size();

    let items: Vec<ListItem> = app
        .items
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Menu"))
        .highlight_symbol("> ");

    frame.render_widget(list, size);
}
