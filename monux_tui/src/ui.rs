use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState,
Paragraph, Wrap},
};


use crate::app::{App, FocusPane};
use crate::markdown::render_markdown_segment;


pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );
    let (panes_area, footer_area, command_area) = if app.command_mode
|| app.search_mode {
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
                    .constraints([Constraint::Percentage(20),
Constraint::Percentage(80)])
                    .split(panes_area);
                (Some(chunks[0]), chunks[1], None)
            }
            (false, true) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(80),
Constraint::Percentage(20)])
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
        if app.search_mode {
            draw_search(frame, app, area);
        } else {
            draw_command(frame, app, area);
        }
    }
    draw_footer(frame, app, footer_area);
    let popup_area = preview_area;
    if app.new_note_popup {
        draw_new_note_popup(frame, app, popup_area);
    }
    if app.global_search_popup {
        draw_global_search_popup(frame, app, popup_area);
    }
    if app.new_dir_popup {
        draw_new_dir_popup(frame, app, popup_area);
    }
    if app.rename_popup {
        draw_rename_popup(frame, app, popup_area);
    }
    if app.delete_dir_confirm_popup {
        draw_delete_dir_confirm_popup(frame, app, popup_area);
    }
    if app.help_popup {
        draw_help_popup(frame, app, popup_area);
    }


    if app.new_note_popup {
        place_new_note_cursor(frame, app, popup_area);
    } else if app.global_search_popup {
        place_global_search_cursor(frame, app, popup_area);
    } else if app.new_dir_popup {
        place_new_dir_cursor(frame, app, popup_area);
    } else if app.rename_popup {
        place_rename_cursor(frame, app, popup_area);
    } else if app.delete_dir_confirm_popup {
    } else if app.help_popup {
    } else if let Some(area) = command_area {
        if app.search_mode {
            place_search_cursor(frame, app, area);
        } else {
            place_command_cursor(frame, app, area);
        }
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
    let paragraph =
Paragraph::new(text).style(Style::default().add_modifier(Modifier::REVERSED)
);
    frame.render_widget(paragraph, area);
}


fn draw_new_note_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_with_offset(area, 74, 11, -2);
    let dir_label = if app.new_note_field ==
crate::app::NewNoteField::Dir {
        "Folder: <-"
    } else {
        "Folder:"
    };
    let name_label = if app.new_note_field ==
crate::app::NewNoteField::Name {
        "Name: <-"
    } else {
        "Name:"
    };
    let tags_label = if app.new_note_field ==
crate::app::NewNoteField::Tags {
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
            "Tab/Shift+Tab: switch field, Enter: create, Esc: cancel",
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("New
Note"))
        .wrap(Wrap { trim: false });
    render_popup_background(frame, popup);
    frame.render_widget(paragraph, popup);
}


fn draw_global_search_popup(frame: &mut Frame, app: &App, area: Rect)
{
    let popup = centered_rect_with_offset(area, 72, 14, -1);
    let mut lines = vec![
        Line::from(format!("Query: {}", app.global_search_input)),
        Line::from(""),
    ];
    for (idx, item) in
app.global_search_results.iter().take(8).enumerate() {
        let (kind, value, kind_color): (&str, String, Color) = match
item {
            crate::app::GlobalSearchResult::Dir(path) => ("DIR ",
path.clone(), Color::Blue),
            crate::app::GlobalSearchResult::Note {
                path,
                matched_by_tag,
            } => {
                let label = path.to_string_lossy().to_string();
                if *matched_by_tag {
                    ("TAG ", label, Color::Yellow)
                } else {
                    ("NOTE", label, Color::Green)
                }
            }
        };
        let prefix = if idx == app.global_search_selected {
            ">"
        } else {
            " "
        };
        lines.push(Line::from(vec![
            Span::styled(prefix.to_string(),
Style::default().fg(Color::Magenta)),
            Span::raw(" ["),
            Span::styled(
                kind.to_string(),

Style::default().fg(kind_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("] "),
            Span::raw(value),
        ]));
    }
    if app.global_search_results.is_empty() {
        lines.push(Line::styled(
            "<no results>",
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "Enter: open, j/k: move, Esc: close",
        Style::default().add_modifier(Modifier::DIM),
    ));


    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Global Search"),
        )
        .wrap(Wrap { trim: false });
    render_popup_background(frame, popup);
    frame.render_widget(paragraph, popup);
}


fn draw_new_dir_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_with_offset(area, 62, 6, -1);
    let text = Text::from(vec![
        Line::from("Path:"),
        Line::from(app.new_dir_input.as_str()),
        Line::from(""),
        Line::styled(
            "Enter: create, Esc: cancel",
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("New Directory"),
        )
        .wrap(Wrap { trim: false });
    render_popup_background(frame, popup);
    frame.render_widget(paragraph, popup);
}


fn draw_rename_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_with_offset(area, 62, 7, -1);
    let text = Text::from(vec![
        Line::from(format!("Target: {}", app.rename_target_label)),
        Line::from(""),
        Line::from("New name/path:"),
        Line::from(app.rename_input.as_str()),
        Line::styled(
            "Enter: apply, Esc: cancel",
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Rename"))
        .wrap(Wrap { trim: false });
    render_popup_background(frame, popup);
    frame.render_widget(paragraph, popup);
}


fn draw_help_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_with_offset(area, 88, 24, -1);
    let lines = vec![
        Line::styled(
            "Monux TUI Help",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            "j/k or arrows: scroll, PgUp/PgDn: page, Home: top, Esc/?:
close",
            Style::default().add_modifier(Modifier::DIM),
        ),
        Line::from(""),
        Line::styled("Global",
Style::default().add_modifier(Modifier::BOLD)),
        Line::from("  ?: open/close this help popup"),
        Line::from("  Tab / Shift+Tab: cycle focus panes"),
        Line::from("  Ctrl+N: create note (dir + name + tags)"),
        Line::from("  Ctrl+F: open global search popup"),
        Line::from("  Ctrl+D: create directory popup"),
        Line::from("  Ctrl+R: rename selected item/current note"),
        Line::from("  Ctrl+S: save current note"),
        Line::from("  q: quit (when no unsaved changes)"),
        Line::from(""),
        Line::styled("Notes Pane",
Style::default().add_modifier(Modifier::BOLD)),
        Line::from("  j/k or arrows: move selection"),
        Line::from("  Enter: open note or enter directory"),
        Line::from("  h/l or arrows: collapse/expand directory"),
        Line::from("  /: search in notes tree"),
        Line::from("  v: visual selection mode"),
        Line::from("  x: cut selected note(s), p: paste/move"),
        Line::from("  D: delete note or selected directory (with
confirm)"),
        Line::from(""),
        Line::styled(
            "Preview Pane",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from("  h/j/k/l or arrows: cursor/motion"),
        Line::from("  3j, 5k, 2h...: numeric count before motion"),
        Line::from("  w/b/e: word motions (start/back/end)"),
        Line::from("  dd: delete line, dw/de/db: delete by word
motion"),
        Line::from("  i/a/o: enter insert mode"),
        Line::from("  v: visual mode (text selection)"),
        Line::from("  z: fold/unfold current markdown heading"),
        Line::from(""),
        Line::styled(
            "Command Mode (:)",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from("  :w                      save current note"),
        Line::from("  :q / :q!                quit / force quit"),
        Line::from("  :mkdir <path>           create directory"),
        Line::from("  :sync [dir]             sync files <-> note
index"),
        Line::from("  :del <query>            delete note by query"),
    ];
    let viewport_lines = popup.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(viewport_lines);
    let scroll = app.help_popup_scroll.min(max_scroll);
    let text = Text::from(
        lines
            .into_iter()
            .skip(scroll)
            .take(viewport_lines)
            .collect::<Vec<_>>(),
    );
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: false });
    render_popup_background(frame, popup);
    frame.render_widget(paragraph, popup);
}


fn draw_delete_dir_confirm_popup(frame: &mut Frame, app: &App, area:
Rect) {
    let popup = centered_rect_with_offset(area, 60, 6, -1);
    let text = Text::from(vec![
        Line::from(format!(
            "Delete directory '{}/'?",
            app.delete_dir_confirm_path
        )),
        Line::from(""),
        Line::styled(
            "Enter/Y: confirm, Esc/N: cancel",
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Delete"),
        )
        .wrap(Wrap { trim: false });
    render_popup_background(frame, popup);
    frame.render_widget(paragraph, popup);
}


fn draw_links(frame: &mut Frame, app: &App, area: Rect) {
    let labels = app.link_labels();
    let links: Vec<ListItem> = if labels.is_empty() {
        vec![ListItem::new(Line::styled(
            "<no links>",
            Style::default().add_modifier(Modifier::DIM),
        ))]
    } else {
        labels
            .iter()
            .map(|label| {
                ListItem::new(Line::styled(
                    label.as_str(),
                    Style::default().fg(Color::Cyan),
                ))
            })
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
    let source_lines = app.preview_source_lines();
    let code_block_rows = fenced_code_block_rows(source_lines);
    let source_len = source_lines.len();


    if source_len == 0 {
        rendered.push(Line::styled(
            "<empty file>",
            Style::default().add_modifier(Modifier::DIM),
        ));
    } else {
        let cursor_row =
app.preview_visible_row_for_source_row(app.cursor_row());
        let end = (scroll + inner_height).min(source_len);


        for idx in scroll..end {
            if app.preview_is_row_hidden(idx) {
                continue;
            }


            let Some(raw_line) = source_lines.get(idx).cloned() else {
                continue;
            };


            let rel = idx.abs_diff(cursor_row);
            let gutter = format!("{:>4} ", rel);


            let mut spans = vec![Span::styled(
                gutter,
                Style::default().add_modifier(Modifier::DIM),
            )];


            if app.preview_is_heading_row(idx) {
                let marker = if app.preview_heading_is_collapsed(idx)
{
                    "▸ "
                } else {
                    "▾ "
                };
                spans.push(Span::styled(
                    marker,
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }


            let in_code_block =
code_block_rows.get(idx).copied().unwrap_or(false);
            if in_code_block {
                let code_style =
Style::default().bg(Color::DarkGray).fg(Color::White);
                if let Some((sel_start, sel_end)) =
                    app.visual_selection_for_row(idx,
raw_line.chars().count())
                {
                    let before = slice_chars(&raw_line, 0, sel_start);
                    let selected = slice_chars(&raw_line, sel_start,
sel_end);
                    let after = slice_chars(&raw_line, sel_end,
raw_line.chars().count());
                    if !before.is_empty() {
                        spans.push(Span::styled(before, code_style));
                    }
                    if !selected.is_empty() {
                        spans.push(Span::styled(
                            selected,

code_style.patch(Style::default().bg(Color::White).fg(Color::Black)),
                        ));
                    }
                    if !after.is_empty() {
                        spans.push(Span::styled(after, code_style));
                    }
                } else {
                    spans.push(Span::styled(raw_line.clone(),
code_style));
                }
            } else {
                if let Some((sel_start, sel_end)) =
                    app.visual_selection_for_row(idx,
raw_line.chars().count())
                {
                    let before = slice_chars(&raw_line, 0, sel_start);
                    let selected = slice_chars(&raw_line, sel_start,
sel_end);
                    let after = slice_chars(&raw_line, sel_end,
raw_line.chars().count());


                    if !before.is_empty() {
                        spans.extend(render_markdown_segment(&before,
Style::default()));
                    }
                    if !selected.is_empty() {
                        let selected_spans =
render_markdown_segment(&selected, Style::default());
                        spans.extend(apply_overlay_style(
                            selected_spans,

Style::default().bg(Color::White).fg(Color::Black),
                        ));
                    }
                    if !after.is_empty() {
                        spans.extend(render_markdown_segment(&after,
Style::default()));
                    }
                } else {
                    spans.extend(render_markdown_segment_with_cursor(
                        &raw_line,
                        Style::default(),
                    ));
                }
            }


            rendered.push(Line::from(spans));
        }
    }


    let preview_title = if app.is_visual_mode() {
        " VISUAL MODE "
    } else {
        ""
    };


    let paragraph = Paragraph::new(Text::from(rendered))
        .block(panel_block(preview_title, app.focus ==
FocusPane::Preview))
        .wrap(Wrap { trim: false });


    frame.render_widget(paragraph, area);
}


fn draw_notes(frame: &mut Frame, app: &App, area: Rect) {
    let labels = app.notes_tree_labels();
    let items: Vec<ListItem> = if labels.is_empty() {
        vec![ListItem::new(Line::styled(
            "<no notes>",
            Style::default().add_modifier(Modifier::DIM),
        ))]
    } else {
        labels
            .into_iter()
            .map(|label| {
                let style = if label == ".." {
                    Style::default().add_modifier(Modifier::DIM)
                } else if label.contains('/') {
                    Style::default().fg(Color::Blue)
                } else if label.starts_with("x ") {
                    Style::default().fg(Color::Yellow)
                } else if label.starts_with("* ") {
                    Style::default().fg(Color::Green)
                } else if label.starts_with("> ") {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default()
                };
                ListItem::new(Line::styled(label, style))
            })
            .collect()
    };


    let mut state = ListState::default();
    if app.notes_tree_len() > 0 {
        state.select(Some(app.selected_note));
    }


    let title = app.notes_tree_title();
    let list = List::new(items)
        .block(panel_block(&title, app.focus == FocusPane::Notes))

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


fn draw_search(frame: &mut Frame, app: &App, area: Rect) {
    let content = format!("/{}", app.search_input);
    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search Notes Tree"),
        )
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


fn place_search_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let input_width = app.search_input.chars().count() as u16;
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
    let row =
app.preview_visible_row_for_source_row(app.cursor_row());
    if row < scroll {
        return;
    }


    let visual_row = app.preview_screen_row_for_source_row(scroll,
row) as u16;
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
    let max_col =
inner_width.saturating_sub(1).saturating_sub(gutter_width);
    let visual_col = col.min(max_col);


    let x = area.x + 1 + gutter_width + visual_col;
    let y = area.y + 1 + visual_row;
    frame.set_cursor_position((x, y));
}


fn place_new_note_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_with_offset(area, 74, 11, -2);
    let (input_width, y) = match app.new_note_field {
        crate::app::NewNoteField::Dir => {
            (app.new_note_dir_input.chars().count() as u16, popup.y +
2)
        }
        crate::app::NewNoteField::Name =>
(app.new_note_input.chars().count() as u16, popup.y + 5),
        crate::app::NewNoteField::Tags => {
            (app.new_note_tags_input.chars().count() as u16, popup.y +
7)
        }
    };
    let max_x = popup.x + popup.width.saturating_sub(2);
    let x = (popup.x + 1 + input_width).min(max_x);
    frame.set_cursor_position((x, y));
}


fn place_global_search_cursor(frame: &mut Frame, app: &App, area:
Rect) {
    let popup = centered_rect_with_offset(area, 72, 14, -1);
    let input_width = app.global_search_input.chars().count() as u16;
    let max_x = popup.x + popup.width.saturating_sub(2);
    let x = (popup.x + 8 + input_width).min(max_x);
    let y = popup.y + 1;
    frame.set_cursor_position((x, y));
}


fn place_new_dir_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_with_offset(area, 62, 6, -1);
    let input_width = app.new_dir_input.chars().count() as u16;
    let max_x = popup.x + popup.width.saturating_sub(2);
    let x = (popup.x + 1 + input_width).min(max_x);
    let y = popup.y + 2;
    frame.set_cursor_position((x, y));
}


fn place_rename_cursor(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect_with_offset(area, 62, 7, -1);
    let input_width = app.rename_input.chars().count() as u16;
    let max_x = popup.x + popup.width.saturating_sub(2);
    let x = (popup.x + 1 + input_width).min(max_x);
    let y = popup.y + 4;
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


fn centered_rect(area: Rect, width_percent: u16, height: u16) -> Rect
{
    let width = ((area.width as u32 * width_percent as u32) / 100) as
u16;
    let popup_width = width.max(20).min(area.width);
    let popup_height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let y = area.y + area.height.saturating_sub(popup_height) / 2;
    Rect::new(x, y, popup_width, popup_height)
}


fn centered_rect_with_offset(area: Rect, width_percent: u16, height:
u16, y_offset: i16) -> Rect {
    let mut popup = centered_rect(area, width_percent, height);
    if y_offset < 0 {
        let shift = (-y_offset) as u16;
        popup.y = popup.y.saturating_sub(shift);
    } else if y_offset > 0 {
        let shift = y_offset as u16;
        let max_y = area.y + area.height.saturating_sub(popup.height);
        popup.y = (popup.y + shift).min(max_y);
    }
    popup
}


fn render_popup_background(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );
}


fn render_markdown_segment_with_cursor(segment: &str, base_style:
Style) -> Vec<Span<'static>> {
    render_markdown_segment(segment, base_style)
}


fn apply_overlay_style(spans: Vec<Span<'static>>, overlay: Style) ->
Vec<Span<'static>> {
    spans
        .into_iter()
        .map(|span| Span::styled(span.content,
span.style.patch(overlay)))
        .collect()
}


fn fenced_code_block_rows(lines: &[String]) -> Vec<bool> {
    let mut rows = vec![false; lines.len()];
    let mut in_block = false;
    for (idx, line) in lines.iter().enumerate() {
        let is_fence = is_fence_line(line);
        if is_fence {
            rows[idx] = true;
            in_block = !in_block;
            continue;
        }
        if in_block {
            rows[idx] = true;
        }
    }
    rows
}


fn is_fence_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```")
}


