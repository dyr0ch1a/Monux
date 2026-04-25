use std::collections::BTreeSet;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use core::context::StorageContext;
use core::editor::{Editor, Event, FsStorage};
use core::index::{NoteIndex, NoteMeta, normalize_slug, note_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Links,
    Preview,
    Notes,
}

pub struct App {
    pub should_quit: bool,
    pub focus: FocusPane,
    pub command_mode: bool,
    pub command_input: String,
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
    editor: Editor<FsStorage>,
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
            status: String::from("tab/shift+tab: switch pane, : command, enter: open, q: quit"),
            notes,
            selected_note: 0,
            links: Vec::new(),
            selected_link: 0,
            editor_lines: Vec::new(),
            editor_scroll: 0,
            current_note_slug: None,
            notes_root: config.notes_dir,
            index,
            editor: Editor::new(FsStorage),
        };

        if !app.notes.is_empty() {
            app.open_selected_note()?;
        }

        Ok(app)
    }

    pub fn on_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.command_mode {
            return self.handle_command_input(key);
        }

        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    FocusPane::Notes => FocusPane::Preview,
                    FocusPane::Preview => FocusPane::Links,
                    FocusPane::Links => FocusPane::Notes,
                }
            }
            KeyCode::BackTab => {
                self.focus = match self.focus {
                    FocusPane::Notes => FocusPane::Links,
                    FocusPane::Preview => FocusPane::Notes,
                    FocusPane::Links => FocusPane::Preview,
                }
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
            }
            KeyCode::Enter => match self.focus {
                FocusPane::Notes => self.open_selected_note()?,
                FocusPane::Links => self.open_selected_link()?,
                FocusPane::Preview => {}
            },
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.execute_editor_command("w")?;
            }
            KeyCode::Char('r') => {
                self.reload_notes()?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_command_input(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                if self.editor.is_in_input_mode() {
                    self.status = "insert mode active; finish with '.'".to_string();
                } else {
                    self.command_mode = false;
                    self.command_input.clear();
                }
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.command_input);
                if self.editor.is_in_input_mode() {
                    match self.editor.execute(&input) {
                        Ok(outcome) => {
                            self.apply_outcome(outcome);
                            if !self.editor.is_in_input_mode() {
                                if let Err(err) = self.refresh_editor_lines() {
                                    self.status = format!("? {err}");
                                } else {
                                    self.refresh_links();
                                }
                                self.command_mode = false;
                            }
                        }
                        Err(err) => {
                            self.status = format!("? {err}");
                        }
                    }
                } else {
                    self.execute_editor_command(input.trim())?;
                    self.command_mode = self.editor.is_in_input_mode();
                }
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

    fn move_down(&mut self) {
        match self.focus {
            FocusPane::Notes => {
                if self.selected_note + 1 < self.notes.len() {
                    self.selected_note += 1;
                }
            }
            FocusPane::Links => {
                if self.selected_link + 1 < self.links.len() {
                    self.selected_link += 1;
                }
            }
            FocusPane::Preview => {
                let max_scroll = self.editor_lines.len().saturating_sub(1) as u16;
                self.editor_scroll = self.editor_scroll.saturating_add(1).min(max_scroll);
            }
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
            FocusPane::Preview => {
                self.editor_scroll = self.editor_scroll.saturating_sub(1);
            }
        }
    }

    fn open_selected_note(&mut self) -> anyhow::Result<()> {
        if self.notes.is_empty() {
            self.status = "notes index is empty".to_string();
            return Ok(());
        }

        let Some(note) = self.notes.get(self.selected_note).cloned() else {
            return Ok(());
        };

        self.open_note_by_slug(&note.slug)
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
            std::fs::File::create(&path)?;
        }

        let outcome = match self.editor.execute(&format!("e {}", path.display())) {
            Ok(outcome) => outcome,
            Err(core::editor::EditorError::UnsavedChanges) => {
                self.status = "unsaved changes; run :w first".to_string();
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };
        self.apply_outcome(outcome);

        self.current_note_slug = Some(note.slug.clone());
        if let Some(pos) = self.notes.iter().position(|n| n.id == note.id) {
            self.selected_note = pos;
        }

        self.status = format!("opened {}", note.slug);
        self.refresh_editor_lines()?;
        self.refresh_links();
        self.editor_scroll = 0;
        Ok(())
    }

    fn execute_editor_command(&mut self, command: &str) -> anyhow::Result<()> {
        if command.is_empty() {
            self.command_mode = false;
            return Ok(());
        }

        let outcome = match self.editor.execute(command) {
            Ok(outcome) => outcome,
            Err(err) => {
                self.status = format!("? {err}");
                return Ok(());
            }
        };
        self.apply_outcome(outcome);

        if !self.editor.is_in_input_mode() {
            if let Err(err) = self.refresh_editor_lines() {
                self.status = format!("? {err}");
            } else {
                self.refresh_links();
            }
        }

        Ok(())
    }

    fn apply_outcome(&mut self, outcome: core::editor::ExecOutcome) {
        for event in outcome.events {
            if let Event::Message(msg) = event {
                self.status = msg;
            }
        }

        if outcome.should_quit {
            self.should_quit = true;
        }
    }

    fn refresh_editor_lines(&mut self) -> anyhow::Result<()> {
        match self.editor.execute("%p") {
            Ok(outcome) => {
                self.editor_lines = outcome
                    .events
                    .into_iter()
                    .filter_map(|event| match event {
                        Event::Line(line) => Some(line),
                        Event::Message(_) => None,
                    })
                    .collect();
                Ok(())
            }
            Err(core::editor::EditorError::EmptyBuffer) => {
                self.editor_lines.clear();
                Ok(())
            }
            Err(err) => Err(err.into()),
        }
    }

    fn refresh_links(&mut self) {
        let mut links = BTreeSet::new();

        for line in &self.editor_lines {
            let mut rest = line.as_str();
            while let Some(start) = rest.find("[[") {
                rest = &rest[start + 2..];
                let Some(end) = rest.find("]]" ) else {
                    break;
                };

                let raw = &rest[..end];
                let slug = normalize_slug(raw);
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

    fn reload_notes(&mut self) -> anyhow::Result<()> {
        self.notes = self.index.list()?;
        if self.selected_note >= self.notes.len() {
            self.selected_note = self.notes.len().saturating_sub(1);
        }
        self.status = format!("loaded {} notes", self.notes.len());
        Ok(())
    }

    pub fn preview_source_lines(&self) -> &[String] {
        &self.editor_lines
    }

    pub fn is_editor_input_mode(&self) -> bool {
        self.editor.is_in_input_mode()
    }

    pub fn notes_tree_labels(&self) -> Vec<String> {
        self.notes
            .iter()
            .map(|note| {
                let depth = note.slug.matches('/').count();
                let leaf = note.slug.rsplit('/').next().unwrap_or(&note.slug);
                let marker = if self.current_note_slug.as_deref() == Some(note.slug.as_str()) {
                    "*"
                } else {
                    " "
                };
                format!("{marker} {}{}", "  ".repeat(depth), leaf)
            })
            .collect()
    }
}
