use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, EditorMode, FocusPane};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let (panes_area, command_area) = if app.command_mode {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);
        (vertical[0], Some(vertical[1]))
    } else {
        (area, None)
    };

    let (notes_area, preview_area, links_area) =
        match (app.show_notes_panel(), app.show_links_panel()) {
            (false, false) => (None, panes_area, None),
            (true, false) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                    .split(panes_area);
                (Some(chunks[0]), chunks[1], None)
            }
            (false, true) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                    .split(panes_area);
                (None, chunks[0], Some(chunks[1]))
            }
            (true, true) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                        Constraint::Percentage(20),
                    ])
                    .split(panes_area);
                (Some(chunks[0]), chunks[1], Some(chunks[2]))
            }
        };

    if let Some(area) = notes_area {
        draw_notes(frame, app, area);
    }
    draw_preview(frame, app, preview_area);
    if let Some(area) = links_area {
        draw_links(frame, app, area);
    }
    if let Some(area) = command_area {
        draw_command(frame, app, area);
    }

    if let Some(area) = command_area {
        place_command_cursor(frame, app, area);
    } else if app.focus == FocusPane::Preview {
        place_editor_cursor(frame, app, preview_area);
    }
}

fn draw_links(frame: &mut Frame, app: &App, area: Rect) {
    let links: Vec<ListItem> = if app.links.is_empty() {
        vec![ListItem::new("<no wiki-links>")]
    } else {
        app.links
            .iter()
            .map(|link| ListItem::new(link.as_str()))
            .collect()
    };

    let mut state = ListState::default();
    if !app.links.is_empty() {
        state.select(Some(app.selected_link));
    }

    let list = List::new(links)
        .block(panel_block("Links", app.focus == FocusPane::Links))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let mode = match app.editor_mode() {
        EditorMode::Normal => "NORMAL",
        EditorMode::Insert => "INSERT",
        EditorMode::Visual => "VISUAL",
    };
    let dirty = if app.is_dirty() { " [+]" } else { "" };
    let title = match &app.current_note_slug {
        Some(slug) => format!("Edit: {slug} [{mode}{dirty}] {}", app.status),
        None => format!("Edit [{mode}{dirty}] {}", app.status),
    };

    let inner_height = area.height.saturating_sub(2) as usize;
    app.ensure_cursor_visible(inner_height);

    let scroll = app.editor_scroll as usize;
    let source = app.preview_source_lines();
    let mut rendered = Vec::new();

    if source.is_empty() {
        rendered.push(Line::styled(
            "<empty file>",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        for (idx, line) in source.iter().enumerate().skip(scroll).take(inner_height) {
            let gutter = format!("{:>4} ", idx + 1);
            let mut spans = vec![Span::styled(gutter, Style::default().fg(Color::DarkGray))];
            let base_style = if idx == app.cursor_row() {
                Style::default().bg(Color::Rgb(30, 30, 30))
            } else {
                Style::default()
            };

            if let Some((sel_start, sel_end)) =
                app.visual_selection_for_row(idx, line.chars().count())
            {
                let before = slice_chars(line, 0, sel_start);
                let selected = slice_chars(line, sel_start, sel_end);
                let after = slice_chars(line, sel_end, line.chars().count());

                if !before.is_empty() {
                    spans.push(Span::styled(before, base_style));
                }
                if !selected.is_empty() {
                    spans.push(Span::styled(
                        selected,
                        base_style.fg(Color::Black).bg(Color::LightYellow),
                    ));
                }
                if !after.is_empty() {
                    spans.push(Span::styled(after, base_style));
                }
                if spans.len() == 1 {
                    spans.push(Span::styled(String::new(), base_style));
                }
            } else {
                spans.push(Span::styled(line.clone(), base_style));
            }

            rendered.push(Line::from(spans));
        }
    }

    let paragraph = Paragraph::new(Text::from(rendered))
        .block(panel_block(&title, app.focus == FocusPane::Preview))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn draw_notes(frame: &mut Frame, app: &App, area: Rect) {
    let labels = app.notes_tree_labels();
    let items: Vec<ListItem> = if labels.is_empty() {
        vec![ListItem::new("<no notes>")]
    } else {
        labels.into_iter().map(ListItem::new).collect()
    };

    let mut state = ListState::default();
    if !app.notes.is_empty() {
        state.select(Some(app.selected_note));
    }

    let list = List::new(items)
        .block(panel_block("Notes Tree", app.focus == FocusPane::Notes))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_command(frame: &mut Frame, app: &App, area: Rect) {
    let content = format!(":{}", app.command_input);

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Command"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn place_command_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let input_width = app.command_input.chars().count() as u16;
    let max_x = area.x + area.width.saturating_sub(2);
    let x = (area.x + 2 + input_width).min(max_x);
    let y = area.y + 1;
    frame.set_cursor_position((x, y));
}

fn place_editor_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);
    if inner_width == 0 || inner_height == 0 {
        return;
    }

    let scroll = app.editor_scroll as usize;
    let row = app.cursor_row();
    if row < scroll {
        return;
    }

    let visual_row = (row - scroll) as u16;
    if visual_row >= inner_height {
        return;
    }

    let gutter_width: u16 = 5;
    let col = app.cursor_col() as u16;
    let max_col = inner_width.saturating_sub(1).saturating_sub(gutter_width);
    let visual_col = col.min(max_col);

    let x = area.x + 1 + gutter_width + visual_col;
    let y = area.y + 1 + visual_row;
    frame.set_cursor_position((x, y));
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(style)
}

fn slice_chars(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}
