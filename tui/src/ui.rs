use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, FocusPane};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(vertical[0]);

    draw_notes(frame, app, panes[0]);
    draw_preview(frame, app, panes[1]);
    draw_links(frame, app, panes[2]);
    draw_command(frame, app, vertical[1]);
}

fn draw_links(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = match &app.current_note_slug {
        Some(slug) => format!("Preview: {slug}"),
        None => "Preview".to_string(),
    };

    let paragraph = Paragraph::new(render_markdown(app.preview_source_lines()))
        .block(panel_block(&title, app.focus == FocusPane::Preview))
        .wrap(Wrap { trim: false })
        .scroll((app.editor_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn draw_notes(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_command(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let prompt = if app.is_editor_input_mode() {
        ">"
    } else {
        ":"
    };

    let content = if app.command_mode {
        format!("{prompt} {}", app.command_input)
    } else {
        app.status.clone()
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Command"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    Block::default().borders(Borders::ALL).title(title).style(style)
}

fn render_markdown(lines: &[String]) -> Text<'static> {
    if lines.is_empty() {
        return Text::from(Line::styled(
            "<empty file>",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let mut rendered = Vec::with_capacity(lines.len());
    let mut in_code_block = false;

    for line in lines {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            rendered.push(Line::styled(
                "──────── code ────────",
                Style::default().fg(Color::DarkGray),
            ));
            continue;
        }

        if in_code_block {
            rendered.push(Line::styled(
                format!("  {line}"),
                Style::default().fg(Color::Green),
            ));
            continue;
        }

        if let Some(rest) = heading(trimmed) {
            rendered.push(Line::styled(
                rest.to_string(),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
            continue;
        }

        if let Some(rest) = list_item(trimmed) {
            let mut spans = vec![Span::styled(
                "• ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )];
            spans.extend(parse_inline_code(rest));
            rendered.push(Line::from(spans));
            continue;
        }

        if let Some(rest) = quote_line(trimmed) {
            let mut spans = vec![Span::styled("│ ", Style::default().fg(Color::Blue))];
            spans.extend(parse_inline_code(rest));
            rendered.push(Line::from(spans));
            continue;
        }

        rendered.push(Line::from(parse_inline_code(line)));
    }

    Text::from(rendered)
}

fn heading(line: &str) -> Option<&str> {
    for prefix in ["###### ", "##### ", "#### ", "### ", "## ", "# "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

fn list_item(line: &str) -> Option<&str> {
    for prefix in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

fn quote_line(line: &str) -> Option<&str> {
    line.strip_prefix("> ")
}

fn parse_inline_code(line: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = line;
    let mut in_code = false;

    while let Some(idx) = rest.find('`') {
        let part = &rest[..idx];
        if !part.is_empty() {
            spans.push(styled_inline(part.to_string(), in_code));
        }

        in_code = !in_code;
        rest = &rest[idx + 1..];
    }

    if !rest.is_empty() {
        spans.push(styled_inline(rest.to_string(), in_code));
    }

    if spans.is_empty() {
        spans.push(Span::raw(String::new()));
    }

    spans
}

fn styled_inline(text: String, in_code: bool) -> Span<'static> {
    if in_code {
        Span::styled(
            text,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(text)
    }
}
