use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let (panes_area, footer_area, command_area) = if app.command_mode {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
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
    let base = format!(" Focus: {focus} | Mode: {mode} | Help: ? ");
    let status = app.status.trim();
    let is_action_result = !status.is_empty()
        && !status.starts_with("-- ")
        && !status.starts_with("leader:")
        && !status.starts_with("d pending")
        && !status.starts_with("y pending")
        && status != "new note";

    let text = if is_action_result {
        format!("{base} {status}")
    } else {
        base
    };
    let paragraph = Paragraph::new(text).style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_widget(paragraph, area);
}

fn draw_new_note_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 60, 8);
    let dir_label = if app.new_note_field == crate::app::NewNoteField::Dir {
        "Folder: <-"
    } else {
        "Folder:"
    };
    let name_label = if app.new_note_field == crate::app::NewNoteField::Name {
        "Name: <-"
    } else {
        "Name:"
    };
    let tags_label = if app.new_note_field == crate::app::NewNoteField::Tags {
        "Tags (comma/space separated): <-"
    } else {
        "Tags (comma/space separated):"
    };
    let text = Text::from(vec![
        Line::from(dir_label),
        Line::from(app.new_note_dir_input.as_str()),
        Line::from(""),
        Line::from(name_label),
        Line::from(app.new_note_input.as_str()),
        Line::from(tags_label),
        Line::from(app.new_note_tags_input.as_str()),
        Line::styled(
            "Tab: switch field, Enter: create, Esc: cancel",
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("New Note"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(area, 72, 12);
    let text = Text::from(vec![
        Line::styled(
            "Key Bindings",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        Line::from("Global:"),
        Line::from("  ?: toggle help"),
        Line::from("  Ctrl+N: new note"),
        Line::from("  Ctrl+S: save"),
        Line::from("  Tab / Shift+Tab: cycle panes"),
        Line::from("  : open command"),
        Line::from("  z: fold/unfold current heading in preview"),
        Line::from("  Notes focus + Shift+D: delete note"),
        Line::from("  :del <title>: delete note by title"),
        Line::from("  :tags add <tags...>: add tags to current note"),
        Line::from("  :tags list: show tags of current note"),
        Line::from(""),
        Line::styled(
            "Esc or ?: close",
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

fn draw_links(frame: &mut Frame, app: &App, area: Rect) {
    let labels = app.link_labels();
    let links: Vec<ListItem> = if labels.is_empty() {
        vec![ListItem::new("<no links>")]
    } else {
        labels
            .iter()
            .map(|label| ListItem::new(label.as_str()))
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
    let mut rendered = Vec::new();
    let source_len = app.preview_source_lines().len();

    if source_len == 0 {
        rendered.push(Line::styled(
            "<empty file>",
            Style::default().add_modifier(Modifier::DIM),
        ));
    } else {
        let cursor_row = app.preview_visible_row_for_source_row(app.cursor_row());
        let end = (scroll + inner_height).min(source_len);
        let mut screen_row = 0usize;

        for idx in scroll..end {
            if app.preview_is_row_hidden(idx) {
                continue;
            }

            let Some(line) = app.preview_source_lines().get(idx).cloned() else {
                continue;
            };

            let rel = idx.abs_diff(cursor_row);
            let gutter = format!("{:>4} ", rel);

            let mut spans = vec![Span::styled(
                gutter,
                Style::default().add_modifier(Modifier::DIM),
            )];

            if app.preview_is_heading_row(idx) {
                let marker = if app.preview_heading_is_collapsed(idx) {
                    "▸ "
                } else {
                    "▾ "
                };
                spans.push(Span::styled(
                    marker,
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }

            if let Some((sel_start, sel_end)) =
                app.visual_selection_for_row(idx, line.chars().count())
            {
                let before = slice_chars(&line, 0, sel_start);
                let selected = slice_chars(&line, sel_start, sel_end);
                let after = slice_chars(&line, sel_end, line.chars().count());

                if !before.is_empty() {
                    spans.extend(render_markdown_segment(&before, Style::default()));
                }
                if !selected.is_empty() {
                    let selected_spans = render_markdown_segment(&selected, Style::default());
                    spans.extend(apply_overlay_style(
                        selected_spans,
                        Style::default().bg(Color::White).fg(Color::Black),
                    ));
                }
                if !after.is_empty() {
                    spans.extend(render_markdown_segment(&after, Style::default()));
                }
            } else {
                spans.extend(render_markdown_segment_with_cursor(&line, Style::default()));
            }

            rendered.push(Line::from(spans));
            screen_row += 1;
        }
    }

    let preview_title = if app.is_visual_mode() {
        " VISUAL MODE "
    } else {
        ""
    };

    let paragraph = Paragraph::new(Text::from(rendered))
        .block(panel_block(preview_title, app.focus == FocusPane::Preview))
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
    if app.notes_tree_len() > 0 {
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
    let row = app.preview_visible_row_for_source_row(app.cursor_row());
    if row < scroll {
        return;
    }

    let visual_row = app.preview_screen_row_for_source_row(scroll, row) as u16;
    if visual_row >= inner_height {
        return;
    }

    let gutter_width: u16 = 5;
    let col = app.cursor_col() as u16
        + if app.preview_is_heading_row(row) {
            2
        } else {
            0
        };
    let max_col = inner_width.saturating_sub(1).saturating_sub(gutter_width);
    let visual_col = col.min(max_col);

    let x = area.x + 1 + gutter_width + visual_col;
    let y = area.y + 1 + visual_row;
    frame.set_cursor_position((x, y));
}

fn place_new_note_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 60, 9);
    let (input_width, y) = match app.new_note_field {
        crate::app::NewNoteField::Dir => {
            (app.new_note_dir_input.chars().count() as u16, popup.y + 2)
        }
        crate::app::NewNoteField::Name => (app.new_note_input.chars().count() as u16, popup.y + 5),
        crate::app::NewNoteField::Tags => {
            (app.new_note_tags_input.chars().count() as u16, popup.y + 7)
        }
    };
    let max_x = popup.x + popup.width.saturating_sub(2);
    let x = (popup.x + 1 + input_width).min(max_x);
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

fn render_markdown_segment(segment: &str, base_style: Style) -> Vec<Span<'static>> {
    render_markdown_segment_with_cursor(segment, base_style)
}

fn render_markdown_segment_with_cursor(segment: &str, base_style: Style) -> Vec<Span<'static>> {
    let trimmed = segment.trim_start();
    let indent_len = segment.len().saturating_sub(trimmed.len());
    let mut styled_base = base_style;

    if trimmed.starts_with('#') {
        styled_base = styled_base.add_modifier(Modifier::BOLD).fg(Color::Cyan);
    } else if trimmed.starts_with("> ") || trimmed == ">" {
        styled_base = styled_base.add_modifier(Modifier::DIM);
    } else if trimmed.starts_with("```") {
        styled_base = styled_base.fg(Color::Yellow);
    }

    let mut out: Vec<Span<'static>> = Vec::new();
    if indent_len > 0 {
        out.push(Span::styled(segment[..indent_len].to_string(), styled_base));
    }

    let text = &segment[indent_len..];
    let mut cursor = 0usize;
    while cursor < text.len() {
        let rest = &text[cursor..];

        if let Some(content) = rest.strip_prefix("`") {
            if let Some(end) = content.find('`') {
                let token = &rest[..end + 2];
                out.push(Span::styled(
                    token.to_string(),
                    styled_base.bg(Color::DarkGray).fg(Color::White),
                ));
                cursor += end + 2;
                continue;
            }
        }

        if let Some(content) = rest.strip_prefix("[[") {
            if let Some(end) = content.find("]]") {
                let token = &rest[..end + 4];
                out.push(Span::styled(
                    token.to_string(),
                    styled_base
                        .fg(Color::Green)
                        .add_modifier(Modifier::UNDERLINED),
                ));
                cursor += end + 4;
                continue;
            }
        }

        let next_inline = rest.find('`');
        let next_wikilink = rest.find("[[");
        let next = match (next_inline, next_wikilink) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => rest.len(),
        };

        let plain = &rest[..next];
        if !plain.is_empty() {
            out.push(Span::styled(plain.to_string(), styled_base));
        }
        cursor += next.max(1);
    }

    if out.is_empty() {
        out.push(Span::styled(String::new(), styled_base));
    }

    out
}

fn apply_overlay_style(spans: Vec<Span<'static>>, overlay: Style) -> Vec<Span<'static>> {
    spans
        .into_iter()
        .map(|span| Span::styled(span.content, span.style.patch(overlay)))
        .collect()
}
