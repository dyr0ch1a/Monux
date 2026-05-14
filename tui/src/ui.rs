use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let (panes_area, footer_area, command_area) = if app.command_mode {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Length(3)])
            .split(area);
        (vertical[0], vertical[1], Some(vertical[2]))
    } else {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        (vertical[0], vertical[1], None)
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
    draw_footer(frame, app, footer_area);
    if app.new_note_popup {
        draw_new_note_popup(frame, app, area);
    }
    if app.help_popup {
        draw_help_popup(frame, area);
    }

    if app.new_note_popup {
        place_new_note_cursor(frame, app, area);
    } else if app.help_popup {
        // no cursor for help popup
    } else if let Some(area) = command_area {
        place_command_cursor(frame, app, area);
    } else if app.focus == FocusPane::Preview {
        place_editor_cursor(frame, app, preview_area);
    }
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let focus = match app.focus {
        FocusPane::Notes => "NOTES",
        FocusPane::Preview => "PREVIEW",
        FocusPane::Links => "LINKS",
    };
    let mode = app.editor_mode_label();
    let text = format!(" Focus: {focus} | Mode: {mode} | Help: ? ");
    let paragraph = Paragraph::new(text).style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_widget(paragraph, area);
}

fn draw_new_note_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 60, 6);
    let text = Text::from(vec![
        Line::from("New note name:"),
        Line::from(app.new_note_input.as_str()),
        Line::from(""),
        Line::styled("Enter: create, Esc: cancel", Style::default().add_modifier(Modifier::DIM)),
    ]);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("New Note"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(area, 72, 12);
    let text = Text::from(vec![
        Line::styled("Key Bindings", Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("Global:"),
        Line::from("  ?: toggle help"),
        Line::from("  Ctrl+N: new note"),
        Line::from("  Ctrl+S: save"),
        Line::from("  Tab / Shift+Tab: cycle panes"),
        Line::from("  : open command"),
        Line::from(""),
        Line::styled("Esc or ?: close", Style::default().add_modifier(Modifier::DIM)),
    ]);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
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
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    app.ensure_cursor_visible(inner_height);

    let scroll = app.editor_scroll as usize;
    let source = app.preview_source_lines();
    let mut rendered = Vec::new();

    if source.is_empty() {
        rendered.push(Line::styled(
            "<empty file>",
            Style::default().add_modifier(Modifier::DIM),
        ));
    } else {
        for (idx, line) in source.iter().enumerate().skip(scroll).take(inner_height) {
            let gutter = format!("{:>4} ", idx + 1);
            let mut spans = vec![Span::styled(gutter, Style::default().add_modifier(Modifier::DIM))];
            let base_style = if idx == app.cursor_row() {
                Style::default().add_modifier(Modifier::REVERSED)
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
                    spans.push(Span::styled(selected, base_style.add_modifier(Modifier::BOLD)));
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
        .block(panel_block("", app.focus == FocusPane::Preview))
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
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
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

fn place_new_note_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 60, 6);
    let input_width = app.new_note_input.chars().count() as u16;
    let max_x = popup.x + popup.width.saturating_sub(2);
    let x = (popup.x + 1 + input_width).min(max_x);
    let y = popup.y + 2;
    frame.set_cursor_position((x, y));
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused {
        Style::default().add_modifier(Modifier::BOLD)
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

fn centered_rect(area: Rect, width_percent: u16, height: u16) -> Rect {
    let width = ((area.width as u32 * width_percent as u32) / 100) as u16;
    let popup_width = width.max(20).min(area.width);
    let popup_height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let y = area.y + area.height.saturating_sub(popup_height) / 2;
    Rect::new(x, y, popup_width, popup_height)
}
