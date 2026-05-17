use std::collections::{BTreeSet, HashSet};
use std::io::ErrorKind;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use core::context::StorageContext;
use core::index::{NoteIndex, NoteMeta, normalize_slug, note_path, parse_tags_input};

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
    Dir { path: String, depth: usize, name: String },
    Note { note_index: usize, depth: usize, name: String },
}

pub struct App {
    pub should_quit: bool,
    pub focus: FocusPane,
    pub command_mode: bool,
    pub command_input: String,
    pub new_note_popup: bool,
    pub new_note_input: String,
    pub new_note_tags_input: String,
    pub new_note_tags_focused: bool,
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
            new_note_input: String::new(),
            new_note_tags_input: String::new(),
            new_note_tags_focused: false,
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

    pub fn on_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.new_note_popup {
            return self.handle_new_note_popup_input(key);
        }
        if self.help_popup {
            return self.handle_help_popup_input(key);
        }
        if self.command_mode {
            return self.handle_command_input(key);
        }

        if self.editor_mode == EditorMode::Normal && self.handle_leader_key(key)? {
            return Ok(());
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s')) {
            self.save_current_note()?;
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('n')) {
            self.new_note_popup = true;
            self.new_note_input.clear();
            self.new_note_tags_input.clear();
            self.new_note_tags_focused = false;
            self.status = "new note".to_string();
            return Ok(());
        }
        if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('?'))
        {
            self.help_popup = true;
            self.pending_leader = false;
            return Ok(());
        }

        if self.focus == FocusPane::Preview {
            if self.editor_mode == EditorMode::Insert {
                self.handle_insert_mode_key(key);
                return Ok(());
            }
            if self.editor_mode == EditorMode::Visual {
                self.handle_visual_mode_key(key)?;
                return Ok(());
            }

            if self.handle_preview_normal_key(key)? {
                return Ok(());
            }
        }

        match key.code {
            KeyCode::Char('q') => {
                if self.dirty {
                    self.status = "unsaved changes; use :q! to discard or :w".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Tab => {
                self.cycle_focus(true);
            }
            KeyCode::BackTab => {
                self.cycle_focus(false);
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                self.pending_leader = false;
            }
            KeyCode::Enter => match self.focus {
                FocusPane::Notes => self.activate_selected_note_tree_row()?,
                FocusPane::Links => self.open_selected_link()?,
                FocusPane::Preview => {}
            },
            KeyCode::Right | KeyCode::Char('l') => {
                if self.focus == FocusPane::Notes {
                    self.expand_selected_dir();
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.focus == FocusPane::Notes {
                    self.collapse_selected_dir_or_parent();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Char('D') => {
                if self.focus == FocusPane::Notes {
                    self.delete_selected_note()?;
                }
            }
            KeyCode::Char('r') => {
                self.reload_notes()?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_leader_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            self.pending_leader = false;
            return Ok(false);
        }

        if self.pending_leader {
            self.pending_leader = false;
            match key.code {
                KeyCode::Char('e') => {
                    self.toggle_notes_panel();
                    return Ok(true);
                }
                KeyCode::Char('b') => {
                    self.toggle_links_panel();
                    return Ok(true);
                }
                KeyCode::Char(' ') => {
                    return Ok(true);
                }
                _ => {
                    return Ok(false);
                }
            }
        }

        if matches!(key.code, KeyCode::Char(' ')) {
            self.pending_leader = true;
            self.status = "leader: <space>e notes, <space>b links".to_string();
            return Ok(true);
        }

        Ok(false)
    }

    fn toggle_notes_panel(&mut self) {
        self.show_notes_panel = !self.show_notes_panel;
        if self.show_notes_panel {
            self.focus = FocusPane::Notes;
            self.status = "notes pane shown".to_string();
        } else {
            if self.focus == FocusPane::Notes {
                self.focus = FocusPane::Preview;
            }
            self.status = "notes pane hidden".to_string();
        }
        self.ensure_focus_visible();
    }

    fn toggle_links_panel(&mut self) {
        self.show_links_panel = !self.show_links_panel;
        if self.show_links_panel {
            self.focus = FocusPane::Links;
            self.status = "links pane shown".to_string();
        } else {
            if self.focus == FocusPane::Links {
                self.focus = FocusPane::Preview;
            }
            self.status = "links pane hidden".to_string();
        }
        self.ensure_focus_visible();
    }

    fn cycle_focus(&mut self, forward: bool) {
        let panes = self.visible_panes();
        if panes.is_empty() {
            self.focus = FocusPane::Preview;
            return;
        }

        let current = panes
            .iter()
            .position(|pane| *pane == self.focus)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % panes.len()
        } else if current == 0 {
            panes.len() - 1
        } else {
            current - 1
        };

        self.focus = panes[next];
    }

    fn visible_panes(&self) -> Vec<FocusPane> {
        let mut panes = Vec::with_capacity(3);
        if self.show_notes_panel {
            panes.push(FocusPane::Notes);
        }
        panes.push(FocusPane::Preview);
        if self.show_links_panel {
            panes.push(FocusPane::Links);
        }
        panes
    }

    fn ensure_focus_visible(&mut self) {
        if (self.focus == FocusPane::Notes && !self.show_notes_panel)
            || (self.focus == FocusPane::Links && !self.show_links_panel)
        {
            self.focus = FocusPane::Preview;
        }
    }

    fn handle_command_input(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_input.clear();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.command_input);
                self.command_mode = false;
                self.execute_command(input.trim())?;
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_new_note_popup_input(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.new_note_popup = false;
                self.new_note_input.clear();
                self.new_note_tags_input.clear();
                self.new_note_tags_focused = false;
                self.status = "new note cancelled".to_string();
            }
            KeyCode::Enter => {
                let name = std::mem::take(&mut self.new_note_input);
                let tags = std::mem::take(&mut self.new_note_tags_input);
                self.new_note_popup = false;
                self.new_note_tags_focused = false;
                if let Err(err) = self.create_new_note(name.trim(), &tags) {
                    self.status = format!("new note error: {err}");
                }
            }
            KeyCode::Tab => {
                self.new_note_tags_focused = !self.new_note_tags_focused;
            }
            KeyCode::Backspace => {
                if self.new_note_tags_focused {
                    self.new_note_tags_input.pop();
                } else {
                    self.new_note_input.pop();
                }
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                if self.new_note_tags_focused {
                    self.new_note_tags_input.push(c);
                } else {
                    self.new_note_input.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_help_popup_input(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
            self.help_popup = false;
        }
        Ok(())
    }

    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        if command.is_empty() {
            return Ok(());
        }

        let mut parts = command.split_whitespace();
        let Some(cmd) = parts.next() else {
            return Ok(());
        };

        match cmd {
            "w" => self.save_current_note()?,
            "q" => {
                if self.dirty {
                    self.status = "unsaved changes; use :q! to discard or :w".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            "q!" => {
                self.should_quit = true;
            }
            "wq" => {
                self.save_current_note()?;
                self.should_quit = true;
            }
            "e" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.is_empty() {
                    self.status = "usage: :e <slug>".to_string();
                } else {
                    self.open_note_by_slug(&rest)?;
                }
            }
            "e!" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.is_empty() {
                    self.status = "usage: :e! <slug>".to_string();
                } else {
                    self.open_note_by_slug_force(&rest, true)?;
                }
            }
            "r" => {
                self.reload_notes()?;
            }
            "del" => {
                let title = parts.collect::<Vec<_>>().join(" ");
                if title.trim().is_empty() {
                    self.status = "usage: :del <title>".to_string();
                } else {
                    self.delete_note_by_title(&title)?;
                }
            }
            "tags" => {
                let sub = parts.next().unwrap_or_default();
                match sub {
                    "add" => {
                        let rest = parts.collect::<Vec<_>>().join(" ");
                        if rest.trim().is_empty() {
                            self.status = "usage: :tags add <tags...>".to_string();
                        } else {
                            self.add_tags_to_current_note(&rest)?;
                        }
                    }
                    "list" => {
                        self.list_tags_for_current_note()?;
                    }
                    _ => {
                        self.status = "usage: :tags <add|list> ...".to_string();
                    }
                }
            }
            _ => {
                self.status = format!("unknown command: {cmd}");
            }
        }

        Ok(())
    }

    fn handle_preview_normal_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.pending_normal_op = None;
                self.status = "-- NORMAL --".to_string();
                return Ok(true);
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                self.pending_normal_op = None;
                return Ok(true);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.move_cursor_right();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor_up();
            }
            KeyCode::Char('b') => {
                self.move_word_backward();
            }
            KeyCode::Char('e') => {
                self.move_word_end_forward();
            }
            KeyCode::Char('0') => {
                self.cursor_col = 0;
            }
            KeyCode::Char('$') => {
                self.cursor_col = self.current_line_len_chars();
            }
            KeyCode::Char('i') => {
                self.enter_insert_mode();
            }
            KeyCode::Char('v') => {
                self.enter_visual_mode();
            }
            KeyCode::Char('u') => {
                self.undo();
            }
            KeyCode::Char('p') => {
                self.paste_after_cursor();
            }
            KeyCode::Char('a') => {
                if self.cursor_col < self.current_line_len_chars() {
                    self.cursor_col += 1;
                }
                self.enter_insert_mode();
            }
            KeyCode::Char('A') => {
                self.cursor_col = self.current_line_len_chars();
                self.enter_insert_mode();
            }
            KeyCode::Char('I') => {
                self.cursor_col = self
                    .current_line_ref()
                    .map(|line| first_non_space_char_idx(line.as_str()))
                    .unwrap_or(0);
                self.enter_insert_mode();
            }
            KeyCode::Char('o') => {
                self.ensure_has_line();
                self.push_undo_snapshot();
                let insert_at = self.cursor_row + 1;
                self.editor_lines.insert(insert_at, String::new());
                self.cursor_row = insert_at;
                self.cursor_col = 0;
                self.dirty = true;
                self.enter_insert_mode();
                self.on_editor_content_changed();
            }
            KeyCode::Char('O') => {
                self.ensure_has_line();
                self.push_undo_snapshot();
                let insert_at = self.cursor_row;
                self.editor_lines.insert(insert_at, String::new());
                self.cursor_col = 0;
                self.dirty = true;
                self.enter_insert_mode();
                self.on_editor_content_changed();
            }
            KeyCode::Char('x') => {
                self.delete_char_under_cursor();
            }
            KeyCode::Char('d') => {
                if self.pending_normal_op == Some('d') {
                    self.delete_current_line();
                    self.pending_normal_op = None;
                } else {
                    self.pending_normal_op = Some('d');
                    self.status = "d pending (use dd to delete line)".to_string();
                }
                self.clamp_cursor();
                return Ok(true);
            }
            KeyCode::Char('y') => {
                if self.pending_normal_op == Some('y') {
                    self.yank_current_line();
                    self.pending_normal_op = None;
                } else {
                    self.pending_normal_op = Some('y');
                    self.status = "y pending (use yy to yank line)".to_string();
                }
                self.clamp_cursor();
                return Ok(true);
            }
            _ => {
                self.pending_normal_op = None;
                return Ok(false);
            }
        }

        self.pending_normal_op = None;
        self.clamp_cursor();
        Ok(true)
    }

    fn handle_visual_mode_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') => {
                self.exit_visual_mode();
                return Ok(());
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                return Ok(());
            }
            KeyCode::Char('h') | KeyCode::Left => self.move_cursor_left(),
            KeyCode::Char('l') | KeyCode::Right => self.move_cursor_right(),
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor_up(),
            KeyCode::Char('b') => self.move_word_backward(),
            KeyCode::Char('e') => self.move_word_end_forward(),
            KeyCode::Char('0') => self.cursor_col = 0,
            KeyCode::Char('$') => self.cursor_col = self.current_line_len_chars(),
            KeyCode::Char('d') | KeyCode::Char('x') => {
                self.delete_visual_selection();
                return Ok(());
            }
            KeyCode::Char('y') => {
                self.yank_visual_selection();
                return Ok(());
            }
            KeyCode::Char('u') => {
                self.undo();
                return Ok(());
            }
            _ => {}
        }

        self.clamp_cursor();
        Ok(())
    }

    fn handle_insert_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.editor_mode = EditorMode::Normal;
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
                self.status = "-- NORMAL --".to_string();
            }
            KeyCode::Enter => {
                self.split_line_at_cursor();
            }
            KeyCode::Backspace => {
                self.backspace_in_insert_mode();
            }
            KeyCode::Delete => {
                self.delete_in_insert_mode();
            }
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Tab => {
                self.insert_char('\t');
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert_char(c);
            }
            _ => {}
        }

        self.clamp_cursor();
    }

    fn move_down(&mut self) {
        match self.focus {
            FocusPane::Notes => {
                if self.selected_note + 1 < self.notes_tree_rows.len() {
                    self.selected_note += 1;
                }
            }
            FocusPane::Links => {
                if self.selected_link + 1 < self.links.len() {
                    self.selected_link += 1;
                }
            }
            FocusPane::Preview => self.move_cursor_down(),
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            FocusPane::Notes => {
                self.selected_note = self.selected_note.saturating_sub(1);
            }
            FocusPane::Links => {
                self.selected_link = self.selected_link.saturating_sub(1);
            }
            FocusPane::Preview => self.move_cursor_up(),
        }
    }

    fn open_selected_note(&mut self) -> anyhow::Result<()> {
        if self.notes_tree_rows.is_empty() {
            self.status = "notes index is empty".to_string();
            return Ok(());
        }

        let Some(note) = self.selected_tree_note().cloned() else {
            return Ok(());
        };

        self.open_note_by_slug(&note.slug)
    }

    fn delete_selected_note(&mut self) -> anyhow::Result<()> {
        if self.notes_tree_rows.is_empty() {
            self.status = "notes index is empty".to_string();
            return Ok(());
        }

        let Some(note) = self.selected_tree_note().cloned() else {
            return Ok(());
        };

        self.delete_note(note)
    }

    fn delete_note_by_title(&mut self, title: &str) -> anyhow::Result<()> {
        let normalized = title.trim();
        let Some(note) = self
            .notes
            .iter()
            .find(|note| note.title == normalized)
            .cloned()
        else {
            self.status = format!("note with title '{}' not found", normalized);
            return Ok(());
        };

        self.delete_note(note)
    }

    fn delete_note(&mut self, note: NoteMeta) -> anyhow::Result<()> {
        if self.dirty {
            self.status = "unsaved changes; save with :w first".to_string();
            return Ok(());
        }

        let path = note_path(&self.notes_root, &note.slug);
        match std::fs::remove_file(&path) {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        self.index.delete(note.id)?;
        self.reload_notes()?;

        if self.current_note_slug.as_deref() == Some(note.slug.as_str()) {
            if self.notes.is_empty() {
                self.current_note_slug = None;
                self.current_note_path = None;
                self.editor_lines = vec![String::new()];
                self.links.clear();
                self.selected_link = 0;
                self.cursor_row = 0;
                self.cursor_col = 0;
                self.editor_scroll = 0;
            } else {
                self.open_selected_note()?;
            }
        }

        self.status = format!("deleted {}", note.slug);
        Ok(())
    }

    fn open_selected_link(&mut self) -> anyhow::Result<()> {
        if self.links.is_empty() {
            return Ok(());
        }

        let Some(slug) = self.links.get(self.selected_link).cloned() else {
            return Ok(());
        };

        self.open_note_by_slug(&slug)
    }

    fn open_note_by_slug(&mut self, slug: &str) -> anyhow::Result<()> {
        self.open_note_by_slug_force(slug, false)
    }

    fn open_note_by_slug_force(&mut self, slug: &str, force: bool) -> anyhow::Result<()> {
        if self.dirty && !force {
            self.status = "unsaved changes; use :w before switching or :e!".to_string();
            return Ok(());
        }

        let normalized = normalize_slug(slug);
        if normalized.is_empty() {
            self.status = "invalid note slug".to_string();
            return Ok(());
        }

        let note = self
            .index
            .get_by_slug(&normalized)?
            .ok_or_else(|| anyhow::anyhow!("note '{}' not found in index", normalized))?;

        let path = note_path(&self.notes_root, &note.slug);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::File::create(&path)?;
        }

        let content = std::fs::read_to_string(&path)?;
        self.editor_lines = split_lines(&content);
        self.ensure_has_line();
        self.current_note_path = Some(path.clone());
        self.current_note_slug = Some(note.slug.clone());
        self.editor_mode = EditorMode::Normal;
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.visual_anchor = None;
        self.editor_scroll = 0;
        self.undo_stack.clear();
        self.dirty = false;

        self.select_note_in_tree_by_id(note.id);

        self.status = format!("opened {}", note.slug);
        self.on_editor_content_changed();
        Ok(())
    }

    fn save_current_note(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.current_note_path.as_ref() else {
            self.status = "no note opened".to_string();
            return Ok(());
        };

        let mut content = self.editor_lines.join("\n");
        if !content.is_empty() {
            content.push('\n');
        }

        std::fs::write(path, content.as_bytes())?;
        self.dirty = false;
        self.status = "written".to_string();
        Ok(())
    }

    fn refresh_links(&mut self) {
        let mut links = BTreeSet::new();

        for line in &self.editor_lines {
            let mut rest = line.as_str();
            while let Some(start) = rest.find("[[") {
                rest = &rest[start + 2..];
                let Some(end) = rest.find("]]") else {
                    break;
                };

                let raw = &rest[..end];
                let target = raw
                    .split_once('|')
                    .map(|(target, _)| target)
                    .unwrap_or(raw)
                    .trim();
                let slug = normalize_slug(target);
                if !slug.is_empty() {
                    links.insert(slug);
                }
                rest = &rest[end + 2..];
            }
        }

        self.links = links.into_iter().collect();
        if self.selected_link >= self.links.len() {
            self.selected_link = self.links.len().saturating_sub(1);
        }
    }

    fn invalidate_preview_prerender(&mut self) {
        self.preview_prerender_valid = false;
    }

    fn on_editor_content_changed(&mut self) {
        self.invalidate_preview_prerender();
        self.refresh_links();
    }

    pub fn prerendered_preview_line(&mut self, idx: usize) -> Option<Vec<Span<'static>>> {
        if !self.preview_prerender_valid {
            self.preview_prerender_lines = self
                .editor_lines
                .iter()
                .map(|line| prerender_markdown_line(line))
                .collect();
            self.preview_prerender_valid = true;
        }

        self.preview_prerender_lines.get(idx).cloned()
    }

    fn reload_notes(&mut self) -> anyhow::Result<()> {
        let selected_slug = self.selected_tree_note().map(|n| n.slug.clone());
        self.notes = self.index.list()?;
        self.rebuild_notes_tree();
        if let Some(slug) = selected_slug {
            self.select_tree_row_by_slug(&slug);
        }
        self.status = format!("loaded {} notes", self.notes.len());
        Ok(())
    }

    fn create_new_note(&mut self, raw_name: &str, raw_tags: &str) -> anyhow::Result<()> {
        if self.dirty {
            self.status = "unsaved changes; save with :w first".to_string();
            return Ok(());
        }

        let normalized = raw_name.trim();
        if normalized.is_empty() {
            self.status = "note name cannot be empty".to_string();
            return Ok(());
        }

        std::fs::create_dir_all(&self.notes_root)?;

        let tags = parse_tags_input(raw_tags);
        let note = self.index.create_note_with_tags(normalized, &tags)?;
        let path = note_path(&self.notes_root, &note.slug);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::File::create(&path)?;
        }

        self.reload_notes()?;
        self.select_note_in_tree_by_id(note.id);
        self.open_note_by_slug_force(&note.slug, true)?;
        if tags.is_empty() {
            self.status = format!("created {}", note.slug);
        } else {
            self.status = format!("created {} with #{}", note.slug, tags.join(" #"));
        }
        Ok(())
    }

    fn add_tags_to_current_note(&mut self, raw_tags: &str) -> anyhow::Result<()> {
        let Some(slug) = self.current_note_slug.clone() else {
            self.status = "no note opened".to_string();
            return Ok(());
        };
        let parsed = parse_tags_input(raw_tags);
        if parsed.is_empty() {
            self.status = "tags are empty".to_string();
            return Ok(());
        }
        let merged = self.index.add_tags_to_slug(&slug, &parsed)?;
        self.status = format!("{} tags: #{}", slug, merged.join(" #"));
        Ok(())
    }

    fn list_tags_for_current_note(&mut self) -> anyhow::Result<()> {
        let Some(slug) = self.current_note_slug.clone() else {
            self.status = "no note opened".to_string();
            return Ok(());
        };
        let tags = self.index.list_tags_by_slug(&slug)?;
        if tags.is_empty() {
            self.status = format!("{} has no tags", slug);
        } else {
            self.status = format!("{} tags: #{}", slug, tags.join(" #"));
        }
        Ok(())
    }

    fn ensure_has_line(&mut self) {
        if self.editor_lines.is_empty() {
            self.editor_lines.push(String::new());
        }
    }

    fn push_undo_snapshot(&mut self) {
        self.undo_stack.push(UndoState {
            lines: self.editor_lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
            dirty: self.dirty,
        });
        const MAX_UNDO_STEPS: usize = 200;
        if self.undo_stack.len() > MAX_UNDO_STEPS {
            self.undo_stack.remove(0);
        }
    }

    fn undo(&mut self) {
        let Some(snapshot) = self.undo_stack.pop() else {
            self.status = "nothing to undo".to_string();
            return;
        };

        self.editor_lines = snapshot.lines;
        self.cursor_row = snapshot.cursor_row;
        self.cursor_col = snapshot.cursor_col;
        self.dirty = snapshot.dirty;
        self.editor_mode = EditorMode::Normal;
        self.visual_anchor = None;
        self.pending_normal_op = None;
        self.clamp_cursor();
        self.on_editor_content_changed();
        self.status = "undo".to_string();
    }

    fn enter_insert_mode(&mut self) {
        self.ensure_has_line();
        self.editor_mode = EditorMode::Insert;
        self.visual_anchor = None;
        self.status = "-- INSERT --".to_string();
    }

    fn enter_visual_mode(&mut self) {
        self.ensure_has_line();
        self.editor_mode = EditorMode::Visual;
        self.visual_anchor = Some((self.cursor_row, self.cursor_col));
        self.status = "-- VISUAL --".to_string();
    }

    fn exit_visual_mode(&mut self) {
        self.editor_mode = EditorMode::Normal;
        self.visual_anchor = None;
        self.status = "-- NORMAL --".to_string();
    }

    fn move_cursor_left(&mut self) {
        self.cursor_col = self.cursor_col.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        let max = self.current_line_len_chars();
        if self.cursor_col < max {
            self.cursor_col += 1;
        }
    }

    fn move_cursor_up(&mut self) {
        self.cursor_row = self.cursor_row.saturating_sub(1);
        self.clamp_cursor_col();
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_row + 1 < self.editor_lines.len() {
            self.cursor_row += 1;
        }
        self.clamp_cursor_col();
    }

    fn move_word_backward(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }

        let mut row = self
            .cursor_row
            .min(self.editor_lines.len().saturating_sub(1));
        let mut col = self.cursor_col;

        loop {
            let chars: Vec<char> = self.editor_lines[row].chars().collect();
            if chars.is_empty() {
                if row == 0 {
                    self.cursor_row = 0;
                    self.cursor_col = 0;
                    return;
                }
                row -= 1;
                col = self.editor_lines[row].chars().count();
                continue;
            }

            let mut i = if col == 0 {
                if row == 0 {
                    self.cursor_row = 0;
                    self.cursor_col = 0;
                    return;
                }
                row -= 1;
                col = self.editor_lines[row].chars().count();
                continue;
            } else {
                col.saturating_sub(1).min(chars.len().saturating_sub(1))
            };

            while i > 0 && chars[i].is_whitespace() {
                i -= 1;
            }

            if chars[i].is_whitespace() {
                if row == 0 {
                    self.cursor_row = 0;
                    self.cursor_col = 0;
                    return;
                }
                row -= 1;
                col = self.editor_lines[row].chars().count();
                continue;
            }

            let class = char_class(chars[i]);
            while i > 0 && char_class(chars[i - 1]) == class {
                i -= 1;
            }

            self.cursor_row = row;
            self.cursor_col = i;
            self.clamp_cursor();
            return;
        }
    }

    fn move_word_end_forward(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }

        let mut row = self
            .cursor_row
            .min(self.editor_lines.len().saturating_sub(1));
        let mut col = self.cursor_col;

        loop {
            let chars: Vec<char> = self.editor_lines[row].chars().collect();
            if chars.is_empty() {
                if row + 1 >= self.editor_lines.len() {
                    self.cursor_row = row;
                    self.cursor_col = 0;
                    return;
                }
                row += 1;
                col = 0;
                continue;
            }

            let mut i = col.min(chars.len().saturating_sub(1));

            while i < chars.len() && !is_word_char(chars[i]) {
                i += 1;
            }

            if i >= chars.len() {
                if row + 1 >= self.editor_lines.len() {
                    self.cursor_row = row;
                    self.cursor_col = chars.len().saturating_sub(1);
                    self.clamp_cursor();
                    return;
                }
                row += 1;
                col = 0;
                continue;
            }

            while i + 1 < chars.len() && is_word_char(chars[i + 1]) {
                i += 1;
            }

            self.cursor_row = row;
            self.cursor_col = i;
            self.clamp_cursor();
            return;
        }
    }

    fn current_line_len_chars(&self) -> usize {
        self.current_line_ref()
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    fn current_line_ref(&self) -> Option<&String> {
        self.editor_lines.get(self.cursor_row)
    }

    fn current_line_mut(&mut self) -> Option<&mut String> {
        self.editor_lines.get_mut(self.cursor_row)
    }

    fn clamp_cursor_col(&mut self) {
        let max = self.current_line_len_chars();
        if self.cursor_col > max {
            self.cursor_col = max;
        }
    }

    fn clamp_cursor(&mut self) {
        if self.editor_lines.is_empty() {
            self.cursor_row = 0;
            self.cursor_col = 0;
            return;
        }

        if self.cursor_row >= self.editor_lines.len() {
            self.cursor_row = self.editor_lines.len() - 1;
        }

        self.clamp_cursor_col();
    }

    fn insert_char(&mut self, ch: char) {
        self.ensure_has_line();
        self.push_undo_snapshot();

        let col = self.cursor_col;
        if let Some(line) = self.current_line_mut() {
            let idx = char_to_byte_idx(line, col);
            if ch == '[' {
                line.insert(idx, '[');
                line.insert(idx + 1, ']');
                self.cursor_col += 1;
            } else {
                line.insert(idx, ch);
                self.cursor_col += 1;
            }
            self.dirty = true;
            self.on_editor_content_changed();
        }
    }

    fn split_line_at_cursor(&mut self) {
        self.ensure_has_line();
        self.push_undo_snapshot();

        let row = self.cursor_row;
        let col = self.cursor_col;
        if row >= self.editor_lines.len() {
            return;
        }

        let idx = char_to_byte_idx(&self.editor_lines[row], col);
        let tail = self.editor_lines[row].split_off(idx);
        self.editor_lines.insert(row + 1, tail);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.dirty = true;
        self.on_editor_content_changed();
    }

    fn backspace_in_insert_mode(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }

        if self.cursor_col > 0 {
            self.push_undo_snapshot();
            let col = self.cursor_col;
            if let Some(line) = self.current_line_mut() {
                let end = char_to_byte_idx(line, col);
                let start = char_to_byte_idx(line, col - 1);
                line.drain(start..end);
                self.cursor_col -= 1;
                self.dirty = true;
                self.on_editor_content_changed();
            }
            return;
        }

        if self.cursor_row > 0 {
            self.push_undo_snapshot();
            let row = self.cursor_row;
            let prev_len = self.editor_lines[row - 1].chars().count();
            let line = self.editor_lines.remove(row);
            self.editor_lines[row - 1].push_str(&line);
            self.cursor_row -= 1;
            self.cursor_col = prev_len;
            self.dirty = true;
            self.on_editor_content_changed();
        }
    }

    fn delete_in_insert_mode(&mut self) {
        self.delete_char_under_cursor();
    }

    fn delete_char_under_cursor(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }

        let row = self.cursor_row;
        if row >= self.editor_lines.len() {
            return;
        }

        let line_len = self.editor_lines[row].chars().count();
        if self.cursor_col < line_len {
            self.push_undo_snapshot();
            let col = self.cursor_col;
            if let Some(line) = self.current_line_mut() {
                let start = char_to_byte_idx(line, col);
                let end = char_to_byte_idx(line, col + 1);
                line.drain(start..end);
                self.dirty = true;
                self.on_editor_content_changed();
            }
        } else if row + 1 < self.editor_lines.len() {
            self.push_undo_snapshot();
            let next = self.editor_lines.remove(row + 1);
            self.editor_lines[row].push_str(&next);
            self.dirty = true;
            self.on_editor_content_changed();
        }

        self.clamp_cursor();
    }

    fn delete_current_line(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }

        self.push_undo_snapshot();
        self.editor_lines.remove(self.cursor_row);
        if self.editor_lines.is_empty() {
            self.editor_lines.push(String::new());
        }

        if self.cursor_row >= self.editor_lines.len() {
            self.cursor_row = self.editor_lines.len().saturating_sub(1);
        }

        self.clamp_cursor_col();
        self.dirty = true;
        self.on_editor_content_changed();
        self.status = "deleted line".to_string();
    }

    fn delete_visual_selection(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) = self.normalized_visual_bounds()
        else {
            self.exit_visual_mode();
            return;
        };

        self.push_undo_snapshot();

        if start_row == end_row {
            if let Some(line) = self.editor_lines.get_mut(start_row) {
                let line_len = line.chars().count();
                let start = start_col.min(line_len);
                let end_exclusive = end_col.saturating_add(1).min(line_len);
                if start < end_exclusive {
                    let start_byte = char_to_byte_idx(line, start);
                    let end_byte = char_to_byte_idx(line, end_exclusive);
                    line.drain(start_byte..end_byte);
                }
            }
        } else if end_row < self.editor_lines.len() {
            let start_line = self.editor_lines[start_row].clone();
            let end_line = self.editor_lines[end_row].clone();

            let start_line_len = start_line.chars().count();
            let end_line_len = end_line.chars().count();

            let prefix = {
                let end = start_col.min(start_line_len);
                let end_byte = char_to_byte_idx(&start_line, end);
                start_line[..end_byte].to_string()
            };

            let suffix = {
                let start = end_col.saturating_add(1).min(end_line_len);
                let start_byte = char_to_byte_idx(&end_line, start);
                end_line[start_byte..].to_string()
            };

            self.editor_lines[start_row] = format!("{prefix}{suffix}");
            self.editor_lines.drain((start_row + 1)..=end_row);
        }

        self.ensure_has_line();
        self.cursor_row = start_row.min(self.editor_lines.len().saturating_sub(1));
        self.cursor_col = start_col;
        self.clamp_cursor();
        self.dirty = true;
        self.on_editor_content_changed();
        self.exit_visual_mode();
        self.status = "deleted selection".to_string();
    }

    fn yank_visual_selection(&mut self) {
        let Some(yank) = self.capture_visual_selection() else {
            self.exit_visual_mode();
            return;
        };

        self.yank_register = Some(yank);
        self.exit_visual_mode();
        self.status = "yanked selection".to_string();
    }

    fn yank_current_line(&mut self) {
        let Some(line) = self.editor_lines.get(self.cursor_row) else {
            self.status = "nothing to yank".to_string();
            return;
        };

        self.yank_register = Some(YankRegister {
            text: line.clone(),
            linewise: true,
        });
        self.status = "yanked line".to_string();
    }

    fn paste_after_cursor(&mut self) {
        let Some(yank) = self.yank_register.clone() else {
            self.status = "register is empty".to_string();
            return;
        };

        self.ensure_has_line();
        self.push_undo_snapshot();

        if yank.linewise {
            let insert_at = (self.cursor_row + 1).min(self.editor_lines.len());
            let mut lines = split_lines(&yank.text);
            if lines.is_empty() {
                lines.push(String::new());
            }

            for (offset, line) in lines.iter().cloned().enumerate() {
                self.editor_lines.insert(insert_at + offset, line);
            }

            self.cursor_row = insert_at;
            self.cursor_col = 0;
        } else {
            self.paste_charwise_after_cursor(&yank.text);
        }

        self.dirty = true;
        self.clamp_cursor();
        self.on_editor_content_changed();
        self.status = "put".to_string();
    }

    fn paste_charwise_after_cursor(&mut self, text: &str) {
        let row = self
            .cursor_row
            .min(self.editor_lines.len().saturating_sub(1));
        let line_len = self.editor_lines[row].chars().count();
        let insert_col = if line_len == 0 {
            0
        } else {
            (self.cursor_col + 1).min(line_len)
        };

        let current = self.editor_lines[row].clone();
        let split_at = char_to_byte_idx(&current, insert_col);
        let prefix = current[..split_at].to_string();
        let suffix = current[split_at..].to_string();
        let parts: Vec<&str> = text.split('\n').collect();

        if parts.len() == 1 {
            self.editor_lines[row] = format!("{prefix}{}{suffix}", parts[0]);
            let inserted = parts[0].chars().count();
            self.cursor_row = row;
            self.cursor_col = if inserted == 0 {
                insert_col
            } else {
                insert_col + inserted - 1
            };
            return;
        }

        self.editor_lines[row] = format!("{prefix}{}", parts[0]);
        let mut insert_row = row + 1;

        for part in &parts[1..parts.len() - 1] {
            self.editor_lines.insert(insert_row, (*part).to_string());
            insert_row += 1;
        }

        let last = parts[parts.len() - 1];
        self.editor_lines
            .insert(insert_row, format!("{last}{suffix}"));
        self.cursor_row = insert_row;
        self.cursor_col = if last.is_empty() {
            0
        } else {
            last.chars().count() - 1
        };
    }

    fn capture_visual_selection(&self) -> Option<YankRegister> {
        let ((start_row, start_col), (end_row, end_col)) = self.normalized_visual_bounds()?;
        let mut out = String::new();

        if start_row == end_row {
            let line = self.editor_lines.get(start_row)?;
            let line_len = line.chars().count();
            let start = start_col.min(line_len);
            let end_exclusive = end_col.saturating_add(1).min(line_len);
            let start_byte = char_to_byte_idx(line, start);
            let end_byte = char_to_byte_idx(line, end_exclusive);
            out.push_str(&line[start_byte..end_byte]);
        } else {
            let start_line = self.editor_lines.get(start_row)?;
            let start_line_len = start_line.chars().count();
            let start_byte = char_to_byte_idx(start_line, start_col.min(start_line_len));
            out.push_str(&start_line[start_byte..]);
            out.push('\n');

            for row in (start_row + 1)..end_row {
                if let Some(line) = self.editor_lines.get(row) {
                    out.push_str(line);
                    out.push('\n');
                }
            }

            let end_line = self.editor_lines.get(end_row)?;
            let end_line_len = end_line.chars().count();
            let end_exclusive = end_col.saturating_add(1).min(end_line_len);
            let end_byte = char_to_byte_idx(end_line, end_exclusive);
            out.push_str(&end_line[..end_byte]);
        }

        Some(YankRegister {
            text: out,
            linewise: false,
        })
    }

    fn normalized_visual_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        if self.editor_mode != EditorMode::Visual {
            return None;
        }

        let anchor = self.visual_anchor?;
        let cursor = (self.cursor_row, self.cursor_col);

        if anchor <= cursor {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }

    fn rebuild_notes_tree(&mut self) {
        let mut dirs = BTreeSet::new();
        for note in &self.notes {
            let parts = note.slug.split('/').collect::<Vec<_>>();
            if parts.len() < 2 {
                continue;
            }
            let mut path = String::new();
            for part in &parts[..parts.len() - 1] {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(part);
                dirs.insert(path.clone());
            }
        }

        let mut rows = Vec::new();
        for dir in dirs {
            let parent_expanded = dir
                .rsplit_once('/')
                .map(|(parent, _)| self.expanded_dirs.contains(parent))
                .unwrap_or(true);
            if !parent_expanded {
                continue;
            }
            let depth = dir.matches('/').count();
            let name = dir
                .rsplit('/')
                .next()
                .map(ToString::to_string)
                .unwrap_or_else(|| dir.clone());
            rows.push(NotesTreeRow::Dir {
                path: dir,
                depth,
                name,
            });
        }

        for (idx, note) in self.notes.iter().enumerate() {
            let parent = note
                .slug
                .rsplit_once('/')
                .map(|(dir, _)| dir.to_string())
                .unwrap_or_default();
            if !self.expanded_dirs.contains(&parent) {
                continue;
            }
            let depth = note.slug.matches('/').count();
            let name = note
                .slug
                .rsplit('/')
                .next()
                .map(ToString::to_string)
                .unwrap_or_else(|| note.slug.clone());
            rows.push(NotesTreeRow::Note {
                note_index: idx,
                depth,
                name,
            });
        }

        self.notes_tree_rows = rows;
        if self.selected_note >= self.notes_tree_rows.len() {
            self.selected_note = self.notes_tree_rows.len().saturating_sub(1);
        }
    }

    fn selected_tree_note(&self) -> Option<&NoteMeta> {
        let row = self.notes_tree_rows.get(self.selected_note)?;
        match row {
            NotesTreeRow::Note { note_index, .. } => self.notes.get(*note_index),
            NotesTreeRow::Dir { .. } => None,
        }
    }

    fn select_note_in_tree_by_id(&mut self, id: u64) {
        if let Some(pos) = self.notes_tree_rows.iter().position(|row| match row {
            NotesTreeRow::Note { note_index, .. } => self
                .notes
                .get(*note_index)
                .map(|n| n.id == id)
                .unwrap_or(false),
            NotesTreeRow::Dir { .. } => false,
        }) {
            self.selected_note = pos;
        }
    }

    fn select_tree_row_by_slug(&mut self, slug: &str) {
        if let Some(pos) = self.notes_tree_rows.iter().position(|row| match row {
            NotesTreeRow::Note { note_index, .. } => self
                .notes
                .get(*note_index)
                .map(|n| n.slug == slug)
                .unwrap_or(false),
            NotesTreeRow::Dir { .. } => false,
        }) {
            self.selected_note = pos;
        }
    }

    fn activate_selected_note_tree_row(&mut self) -> anyhow::Result<()> {
        let Some(row) = self.notes_tree_rows.get(self.selected_note).cloned() else {
            return Ok(());
        };
        match row {
            NotesTreeRow::Dir { path, .. } => {
                if self.expanded_dirs.contains(&path) {
                    self.expanded_dirs.remove(&path);
                    self.status = format!("collapsed {path}/");
                } else {
                    self.expanded_dirs.insert(path.clone());
                    self.status = format!("expanded {path}/");
                }
                self.rebuild_notes_tree();
            }
            NotesTreeRow::Note { note_index, .. } => {
                if let Some(note) = self.notes.get(note_index).cloned() {
                    self.open_note_by_slug(&note.slug)?;
                }
            }
        }
        Ok(())
    }

    fn expand_selected_dir(&mut self) {
        let Some(NotesTreeRow::Dir { path, .. }) = self.notes_tree_rows.get(self.selected_note) else {
            return;
        };
        let path = path.clone();
        if self.expanded_dirs.insert(path.clone()) {
            self.rebuild_notes_tree();
            self.status = format!("expanded {path}/");
        }
    }

    fn collapse_selected_dir_or_parent(&mut self) {
        let Some(row) = self.notes_tree_rows.get(self.selected_note).cloned() else {
            return;
        };

        match row {
            NotesTreeRow::Dir { path, .. } => {
                if self.expanded_dirs.remove(&path) {
                    self.rebuild_notes_tree();
                    self.status = format!("collapsed {path}/");
                } else if let Some((parent, _)) = path.rsplit_once('/') {
                    self.select_dir_row(parent);
                }
            }
            NotesTreeRow::Note { note_index, .. } => {
                if let Some(note) = self.notes.get(note_index) {
                    if let Some((parent, _)) = note.slug.rsplit_once('/') {
                        let parent = parent.to_string();
                        self.select_dir_row(&parent);
                    }
                }
            }
        }
    }

    fn select_dir_row(&mut self, dir: &str) {
        if let Some(pos) = self.notes_tree_rows.iter().position(|row| match row {
            NotesTreeRow::Dir { path, .. } => path == dir,
            NotesTreeRow::Note { .. } => false,
        }) {
            self.selected_note = pos;
        }
    }

    pub fn preview_source_lines(&self) -> &[String] {
        &self.editor_lines
    }

    pub fn editor_mode_label(&self) -> &'static str {
        match self.editor_mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual => "VISUAL",
        }
    }

    pub fn is_visual_mode(&self) -> bool {
        self.editor_mode == EditorMode::Visual
    }

    pub fn notes_tree_labels(&self) -> Vec<String> {
        self.notes_tree_rows
            .iter()
            .map(|row| match row {
                NotesTreeRow::Dir { path, depth, name } => {
                    let icon = if self.expanded_dirs.contains(path) {
                        "▾"
                    } else {
                        "▸"
                    };
                    format!("  {}{} {name}/", "  ".repeat(*depth), icon)
                }
                NotesTreeRow::Note { note_index, depth, name } => {
                    let note = &self.notes[*note_index];
                    let marker = if self.current_note_slug.as_deref() == Some(note.slug.as_str()) {
                        "*"
                    } else {
                        " "
                    };
                    format!("{marker} {}{name}", "  ".repeat(*depth))
                }
            })
            .collect()
    }

    pub fn notes_tree_len(&self) -> usize {
        self.notes_tree_rows.len()
    }

    pub fn link_labels(&self) -> Vec<String> {
        self.links
            .iter()
            .map(|slug| {
                self.notes
                    .iter()
                    .find(|note| note.slug == *slug)
                    .map(|note| note.title.clone())
                    .unwrap_or_else(|| slug.clone())
            })
            .collect()
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    pub fn visual_selection_for_row(&self, row: usize, line_len: usize) -> Option<(usize, usize)> {
        let ((start_row, start_col), (end_row, end_col)) = self.normalized_visual_bounds()?;
        if row < start_row || row > end_row {
            return None;
        }

        let (mut from, mut to) = if start_row == end_row {
            (start_col, end_col.saturating_add(1))
        } else if row == start_row {
            (start_col, line_len)
        } else if row == end_row {
            (0, end_col.saturating_add(1))
        } else {
            (0, line_len)
        };

        from = from.min(line_len);
        to = to.min(line_len);
        if from >= to { None } else { Some((from, to)) }
    }

    pub fn show_notes_panel(&self) -> bool {
        self.show_notes_panel
    }

    pub fn show_links_panel(&self) -> bool {
        self.show_links_panel
    }

    pub fn ensure_cursor_visible(&mut self, view_height: usize) {
        if view_height == 0 {
            return;
        }

        let top = self.editor_scroll as usize;
        let bottom = top + view_height.saturating_sub(1);

        if self.cursor_row < top {
            self.editor_scroll = self.cursor_row as u16;
        } else if self.cursor_row > bottom {
            let new_top = self
                .cursor_row
                .saturating_sub(view_height.saturating_sub(1));
            self.editor_scroll = new_top as u16;
        }
    }
}

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
    let trimmed = line.trim_start();
    let indent_len = line.len().saturating_sub(trimmed.len());
    let mut styled_base = Style::default();

    if trimmed.starts_with('#') {
        styled_base = styled_base.add_modifier(Modifier::BOLD).fg(Color::Cyan);
    } else if trimmed.starts_with("> ") || trimmed == ">" {
        styled_base = styled_base.add_modifier(Modifier::DIM);
    } else if trimmed.starts_with("```") {
        styled_base = styled_base.fg(Color::Yellow);
    }

    let mut out: Vec<Span<'static>> = Vec::new();
    if indent_len > 0 {
        out.push(Span::styled(line[..indent_len].to_string(), styled_base));
    }

    let text = &line[indent_len..];
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
