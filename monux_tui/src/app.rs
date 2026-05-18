use std::collections::{BTreeSet, HashSet};
use std::io::ErrorKind;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use monux_core::context::StorageContext;
use monux_core::index::{normalize_slug, note_path, parse_tags_input, NoteIndex, NoteMeta};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Links,
    Preview,
    Notes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Visual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewNoteField {
    Dir,
    Name,
    Tags,
}

#[derive(Clone)]
struct UndoState {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    dirty: bool,
}

#[derive(Clone)]
struct YankRegister {
    text: String,
    linewise: bool,
}

#[derive(Debug, Clone)]
enum NotesTreeRow {
    Dir {
        path: String,
        depth: usize,
        name: String,
    },
    Note {
        note_index: usize,
        depth: usize,
        name: String,
    },
}

pub struct App {
    pub should_quit: bool,
    pub focus: FocusPane,
    pub command_mode: bool,
    pub command_input: String,
    pub new_note_popup: bool,
    pub new_note_dir_input: String,
    pub new_note_input: String,
    pub new_note_tags_input: String,
    pub new_note_field: NewNoteField,
    pub help_popup: bool,
    pub status: String,
    pub notes: Vec<NoteMeta>,
    pub selected_note: usize,
    pub links: Vec<String>,
    pub selected_link: usize,
    pub editor_lines: Vec<String>,
    pub editor_scroll: u16,
    pub current_note_slug: Option<String>,

    notes_root: PathBuf,
    index: NoteIndex,
    current_note_path: Option<PathBuf>,
    editor_mode: EditorMode,
    cursor_row: usize,
    cursor_col: usize,
    visual_anchor: Option<(usize, usize)>,
    pending_normal_op: Option<char>,
    pending_leader: bool,
    show_notes_panel: bool,
    show_links_panel: bool,
    undo_stack: Vec<UndoState>,
    yank_register: Option<YankRegister>,
    dirty: bool,
    preview_prerender_valid: bool,
    preview_prerender_lines: Vec<Vec<Span<'static>>>,
    notes_tree_rows: Vec<NotesTreeRow>,
    expanded_dirs: HashSet<String>,
    collapsed_headings: HashSet<usize>,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let storage = StorageContext::new()?;
        let config = storage.load_config()?;
        let index = storage.open_note_index()?;
        let notes = index.list()?;

        let mut app = Self {
            should_quit: false,
            focus: FocusPane::Preview,
            command_mode: false,
            command_input: String::new(),
            new_note_popup: false,
            new_note_dir_input: String::new(),
            new_note_input: String::new(),
            new_note_tags_input: String::new(),
            new_note_field: NewNoteField::Name,
            help_popup: false,
            status: String::from(
                "tab/shift+tab: pane, h/j/k/l: move, i/a/o: insert, : for commands, ctrl+s: save",
            ),
            notes,
            selected_note: 0,
            links: Vec::new(),
            selected_link: 0,
            editor_lines: Vec::new(),
            editor_scroll: 0,
            current_note_slug: None,
            notes_root: config.notes_dir,
            index,
            current_note_path: None,
            editor_mode: EditorMode::Normal,
            cursor_row: 0,
            cursor_col: 0,
            visual_anchor: None,
            pending_normal_op: None,
            pending_leader: false,
            show_notes_panel: false,
            show_links_panel: false,
            undo_stack: Vec::new(),
            yank_register: None,
            dirty: false,
            preview_prerender_valid: false,
            preview_prerender_lines: Vec::new(),
            notes_tree_rows: Vec::new(),
            expanded_dirs: HashSet::new(),
            collapsed_headings: HashSet::new(),
        };

        app.expanded_dirs.insert(String::new());
        app.rebuild_notes_tree();
        if let Some(pos) = app
            .notes_tree_rows
            .iter()
            .position(|row| matches!(row, NotesTreeRow::Note { .. }))
        {
            app.selected_note = pos;
        }

        if !app.notes.is_empty() {
            app.open_selected_note()?;
        }

        Ok(app)
    }
}

include!("app/key_input.rs");
include!("app/notes.rs");
include!("app/editor.rs");
include!("app/tree.rs");
include!("app/preview_b.rs");

fn split_lines(content: &str) -> Vec<String> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<String> = content.split('\n').map(|line| line.to_string()).collect();
    if content.ends_with('\n') {
        lines.pop();
    }

    lines
}

fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}

fn first_non_space_char_idx(line: &str) -> usize {
    line.chars().position(|ch| !ch.is_whitespace()).unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Word,
    Symbol,
    Whitespace,
}

fn char_class(ch: char) -> CharClass {
    if ch.is_whitespace() {
        CharClass::Whitespace
    } else if ch.is_ascii_alphanumeric() || ch == '_' {
        CharClass::Word
    } else {
        CharClass::Symbol
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn prerender_markdown_line(line: &str) -> Vec<Span<'static>> {
    let mut base = Style::default();
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        base = base.fg(Color::Cyan).add_modifier(Modifier::BOLD);
    } else if trimmed.starts_with("> ") || trimmed == ">" {
        base = base.add_modifier(Modifier::DIM);
    } else if trimmed.starts_with("```") {
        base = base.fg(Color::Yellow);
    }

    let mut out: Vec<Span<'static>> = Vec::new();
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let parser = Parser::new_ext(line, options);
    let mut style_stack: Vec<Style> = vec![base];

    for event in parser {
        match event {
            Event::Start(tag) => {
                let current = *style_stack.last().unwrap_or(&base);
                let next = match tag {
                    Tag::Emphasis => current.add_modifier(Modifier::ITALIC),
                    Tag::Strong => current.add_modifier(Modifier::BOLD),
                    Tag::Strikethrough => current.add_modifier(Modifier::CROSSED_OUT),
                    Tag::CodeBlock(_) => current.bg(Color::DarkGray).fg(Color::White),
                    Tag::Link { .. } => current.fg(Color::Green).add_modifier(Modifier::UNDERLINED),
                    Tag::Image { .. } => current.fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    _ => current,
                };
                style_stack.push(next);
            }
            Event::End(TagEnd::CodeBlock) => {
                style_stack.pop();
            }
            Event::End(_) => {
                style_stack.pop();
            }
            Event::Code(text) => {
                let style = *style_stack.last().unwrap_or(&base);
                out.push(Span::styled(
                    format!("`{text}`"),
                    style.bg(Color::DarkGray).fg(Color::White),
                ));
            }
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                let style = *style_stack.last().unwrap_or(&base);
                out.push(Span::styled(text.to_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => {
                let style = *style_stack.last().unwrap_or(&base);
                out.push(Span::styled(" ".to_string(), style));
            }
            Event::Rule => {
                let style = *style_stack.last().unwrap_or(&base);
                out.push(Span::styled("─".to_string(), style));
            }
            Event::TaskListMarker(done) => {
                let style = *style_stack.last().unwrap_or(&base);
                let marker = if done { "[x] " } else { "[ ] " };
                out.push(Span::styled(marker.to_string(), style));
            }
            Event::FootnoteReference(name) => {
                let style = *style_stack.last().unwrap_or(&base);
                out.push(Span::styled(
                    format!("[^{name}]"),
                    style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                ));
            }
            _ => {}
        }
    }

    if out.is_empty() {
        out.push(Span::styled(line.to_string(), base));
    }
    out
}
