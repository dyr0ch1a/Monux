use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use monux_core::context::StorageContext;
use monux_core::fsstorage::storage::NoteStorage;
use monux_core::fsstorage::watch::VaultWatcher;
use monux_core::index::{NoteIndex, NoteMeta, parse_tags_input, path_key};

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
    ParentDir {
        path: String,
    },
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

#[derive(Debug, Clone)]
pub enum GlobalSearchResult {
    Dir(String),
    Note { path: PathBuf, matched_by_tag: bool },
}

#[derive(Debug, Clone)]
enum RenameTarget {
    Note(PathBuf),
    Dir(String),
}

pub struct App {
    pub should_quit: bool,
    pub focus: FocusPane,
    pub command_mode: bool,
    pub command_input: String,
    pub search_mode: bool,
    pub search_input: String,
    pub last_note_search: String,
    pub new_note_popup: bool,
    pub global_search_popup: bool,
    pub new_dir_popup: bool,
    pub rename_popup: bool,
    pub delete_dir_confirm_popup: bool,
    pub new_dir_input: String,
    pub rename_input: String,
    pub rename_target_label: String,
    pub delete_dir_confirm_path: String,
    pub new_note_dir_input: String,
    pub new_note_input: String,
    pub new_note_tags_input: String,
    pub global_search_input: String,
    pub global_search_selected: usize,
    pub global_search_results: Vec<GlobalSearchResult>,
    pub new_note_field: NewNoteField,
    pub help_popup: bool,
    pub help_popup_scroll: usize,
    pub status: String,
    pub notes: Vec<NoteMeta>,
    pub selected_note: usize,
    pub links: Vec<PathBuf>,
    pub selected_link: usize,
    pub editor_lines: Vec<String>,
    pub editor_scroll: u16,
    pub current_note_rel: Option<PathBuf>,

    notes_root: PathBuf,
    autosave_enabled: bool,
    index: NoteIndex,
    storage: NoteStorage,
    note_buffers: HashMap<PathBuf, NoteBuffer>,
    note_buffer_lru: VecDeque<PathBuf>,
    current_note_path: Option<PathBuf>,
    state_path: PathBuf,
    editor_mode: EditorMode,
    cursor_row: usize,
    cursor_col: usize,
    visual_anchor: Option<(usize, usize)>,
    visual_line_mode: bool,
    pending_normal_op: Option<char>,
    motion_count: Option<usize>,
    pending_leader: bool,
    show_notes_panel: bool,
    show_links_panel: bool,
    undo_stack: Vec<UndoState>,
    yank_register: Option<YankRegister>,
    dirty: bool,
    notes_tree_rows: Vec<NotesTreeRow>,
    current_notes_dir: String,
    expanded_dirs: HashSet<String>,
    collapsed_headings: HashSet<usize>,
    notes_tree_visual_mode: bool,
    notes_tree_visual_anchor: Option<usize>,
    notes_tree_cut_paths: Vec<PathBuf>,
    vault_watcher: Option<VaultWatcher>,
    rename_target: Option<RenameTarget>,
    pending_links_refresh: bool,
    pending_autosave: bool,
    last_content_change_at: Instant,
    last_undo_snapshot_at: Option<Instant>,
    last_autosave_rel: Option<PathBuf>,
    last_autosave_at: Option<Instant>,
    notes_loaded: bool,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let storage = StorageContext::new()?;
        let config = storage.load_config()?;
        let index = storage.open_note_index()?;
        let note_storage = storage.open_note_storage()?;
        let vault_watcher = storage.open_vault_watcher().ok();
        let state_path = storage
            .config_path()
            .parent()
            .map(|p| p.join("tui_state.toml"))
            .unwrap_or_else(|| PathBuf::from("tui_state.toml"));

        let mut app = Self {
            should_quit: false,
            focus: FocusPane::Preview,
            command_mode: false,
            command_input: String::new(),
            search_mode: false,
            search_input: String::new(),
            last_note_search: String::new(),
            new_note_popup: false,
            global_search_popup: false,
            new_dir_popup: false,
            rename_popup: false,
            delete_dir_confirm_popup: false,
            new_dir_input: String::new(),
            rename_input: String::new(),
            rename_target_label: String::new(),
            delete_dir_confirm_path: String::new(),
            new_note_dir_input: String::new(),
            new_note_input: String::new(),
            new_note_tags_input: String::new(),
            global_search_input: String::new(),
            global_search_selected: 0,
            global_search_results: Vec::new(),
            new_note_field: NewNoteField::Name,
            help_popup: false,
            help_popup_scroll: 0,
            status: String::from(
                "tab/shift+tab: pane, h/j/k/l: move, i/a/o: insert, : for commands, ctrl+s: save",
            ),
            notes: Vec::new(),
            selected_note: 0,
            links: Vec::new(),
            selected_link: 0,
            editor_lines: Vec::new(),
            editor_scroll: 0,
            current_note_rel: None,
            notes_root: config.notes_dir,
            autosave_enabled: config.autosave,
            index,
            storage: note_storage,
            note_buffers: HashMap::new(),
            note_buffer_lru: VecDeque::new(),
            current_note_path: None,
            state_path,
            editor_mode: EditorMode::Normal,
            cursor_row: 0,
            cursor_col: 0,
            visual_anchor: None,
            visual_line_mode: false,
            pending_normal_op: None,
            motion_count: None,
            pending_leader: false,
            show_notes_panel: false,
            show_links_panel: false,
            undo_stack: Vec::new(),
            yank_register: None,
            dirty: false,
            notes_tree_rows: Vec::new(),
            current_notes_dir: String::new(),
            expanded_dirs: HashSet::new(),
            collapsed_headings: HashSet::new(),
            notes_tree_visual_mode: false,
            notes_tree_visual_anchor: None,
            notes_tree_cut_paths: Vec::new(),
            vault_watcher,
            rename_target: None,
            pending_links_refresh: false,
            pending_autosave: false,
            last_content_change_at: Instant::now(),
            last_undo_snapshot_at: None,
            last_autosave_rel: None,
            last_autosave_at: None,
            notes_loaded: false,
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

        Ok(app)
    }
}

impl App {
    pub fn load_notes_if_needed(&mut self) -> anyhow::Result<()> {
        if self.notes_loaded {
            return Ok(());
        }
        self.reload_notes_internal(false)?;
        self.notes_loaded = true;
        if let Some(rel) = self.load_last_opened_path()
            && self.notes.iter().any(|n| n.path == rel)
        {
            let _ = self.open_note_by_path_force(&rel, true);
        }
        Ok(())
    }

    pub fn poll_filesystem_updates(&mut self) -> anyhow::Result<()> {
        if !self.notes_loaded {
            return Ok(());
        }
        let Some(watcher) = &self.vault_watcher else {
            return Ok(());
        };

        let paths = watcher.drain();
        if paths.is_empty() {
            return Ok(());
        }

        let mut reloaded = false;
        for abs in paths {
            if abs.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Ok(rel) = abs.strip_prefix(&self.notes_root) {
                if self.should_ignore_watcher_event(rel) {
                    continue;
                }
                self.index.reindex_note(rel)?;
                reloaded = true;
            }
        }
        if reloaded {
            self.reload_notes()?;
        }
        Ok(())
    }

    pub fn on_tick(&mut self) {
        let now = Instant::now();
        if self.pending_links_refresh
            && now.duration_since(self.last_content_change_at) >= Duration::from_millis(80)
        {
            self.refresh_links();
            self.pending_links_refresh = false;
        }
        if self.pending_autosave
            && self.autosave_enabled
            && self.dirty
            && now.duration_since(self.last_content_change_at) >= Duration::from_millis(450)
        {
            if let Err(err) = self.autosave_current_note() {
                self.status = format!("autosave error: {err}");
            }
            self.pending_autosave = false;
        }
    }

    fn should_ignore_watcher_event(&self, rel: &std::path::Path) -> bool {
        let (Some(last_rel), Some(last_at)) = (&self.last_autosave_rel, self.last_autosave_at)
        else {
            return false;
        };
        rel == last_rel && Instant::now().duration_since(last_at) < Duration::from_millis(1200)
    }
}

include!("app/buffer.rs");
include!("app/dirs.rs");
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
    } else if ch.is_alphanumeric() || ch == '_' {
        CharClass::Word
    } else {
        CharClass::Symbol
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn prettify_segment(input: &str) -> String {
    let normalized = input
        .trim()
        .replace(['-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for (idx, word) in normalized.split_whitespace().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

fn display_name_from_note_path(path: &str) -> String {
    let leaf = path.rsplit('/').next().unwrap_or(path).trim();
    let pretty = prettify_segment(leaf);
    if pretty.is_empty() {
        leaf.to_string()
    } else {
        pretty
    }
}
