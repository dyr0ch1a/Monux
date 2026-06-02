impl App {
    fn parent_dir_of(path: &str) -> String {
        path.rsplit_once('/')
            .map(|(parent, _)| parent.to_string())
            .unwrap_or_default()
    }


    fn enter_notes_dir(&mut self, dir: &str) {
        self.current_notes_dir = dir.to_string();
        self.rebuild_notes_tree();
        self.selected_note = 0;
    }


    fn apply_notes_tree_search(&mut self) {
        let q = self.search_input.trim().to_lowercase();
        if q.is_empty() {
            return;
        }
        if let Some((idx, _)) =
self.notes_tree_rows.iter().enumerate().find(|(_, row)| match row {
            NotesTreeRow::ParentDir { path } =>
path.to_lowercase().contains(&q),
            NotesTreeRow::Dir { path, name, .. } => {
                path.to_lowercase().contains(&q) ||
name.to_lowercase().contains(&q)
            }
            NotesTreeRow::Note { note_index, name, .. } => self
                .notes
                .get(*note_index)
                .map(|n| {
                    n.display_path().to_lowercase().contains(&q)
                        || n.title.to_lowercase().contains(&q)
                        || name.to_lowercase().contains(&q)
                })
                .unwrap_or(false),
        }) {
            self.selected_note = idx;
        }
    }


    fn notes_tree_selection_bounds(&self) -> Option<(usize, usize)> {
        if !self.notes_tree_visual_mode {
            return None;
        }
        let anchor = self.notes_tree_visual_anchor?;
        let start = anchor.min(self.selected_note);
        let end = anchor.max(self.selected_note);
        Some((start, end))
    }


    fn notes_tree_selected_note_paths(&self) -> Vec<PathBuf> {
        if let Some((start, end)) = self.notes_tree_selection_bounds()
{
            let mut out = Vec::new();
            for idx in start..=end {
                if let Some(NotesTreeRow::Note { note_index, .. }) =
self.notes_tree_rows.get(idx) {
                    if let Some(note) = self.notes.get(*note_index) {
                        out.push(note.path.clone());
                    }
                }
            }
            out
        } else {
            self.selected_tree_note()
                .map(|note| vec![note.path.clone()])
                .unwrap_or_default()
        }
    }


    fn enter_notes_tree_visual_mode(&mut self) {
        self.notes_tree_visual_mode = true;
        self.notes_tree_visual_anchor = Some(self.selected_note);
        self.status = "-- NOTES VISUAL --".to_string();
    }


    fn exit_notes_tree_visual_mode(&mut self) {
        self.notes_tree_visual_mode = false;
        self.notes_tree_visual_anchor = None;
        self.status = "-- NOTES NORMAL --".to_string();
    }


    fn cut_notes_tree_selection(&mut self) {
        let paths = self.notes_tree_selected_note_paths();
        if paths.is_empty() {
            self.status = "no notes selected to cut".to_string();
            return;
        }
        self.notes_tree_cut_paths = paths;
        if self.notes_tree_visual_mode {
            self.exit_notes_tree_visual_mode();
        }
        self.status = format!(
            "cut {} note(s); select target dir and press p",
            self.notes_tree_cut_paths.len()
        );
    }


    fn notes_tree_target_dir(&self) -> Option<String> {
        match self.notes_tree_rows.get(self.selected_note) {
            // For paste/move operations, `..` means "current directory context",
            // not its parent.
            Some(NotesTreeRow::ParentDir { .. }) => Some(self.current_notes_dir.clone()),
            Some(NotesTreeRow::Dir { path, .. }) =>
Some(path.clone()),
            Some(NotesTreeRow::Note { note_index, .. }) => self
                .notes
                .get(*note_index)
                .map(|note| {
                    note.path
                        .parent()
                        .and_then(|p| p.to_str())
                        .map(str::to_string)
                        .unwrap_or_default()
                }),
            None => Some(String::new()),
        }
    }


    fn paste_cut_notes_to_selected_dir(&mut self) ->
anyhow::Result<()> {
        if self.notes_tree_cut_paths.is_empty() {
            self.status = "cut buffer is empty".to_string();
            return Ok(());
        }


        let Some(target_dir) = self.notes_tree_target_dir() else {
            self.status = "cannot resolve target
directory".to_string();
            return Ok(());
        };


        let paths = std::mem::take(&mut self.notes_tree_cut_paths);
        let mut moved = 0usize;
        let mut failed = Vec::new();
        let mut last_error: Option<String> = None;
        for rel in &paths {
            let Some(note) = self.notes.iter().find(|n| n.path ==
*rel).cloned() else {
                continue;
            };
            if let Err(err) = self.move_note_to_dir(note, &target_dir) {
                last_error = Some(err.to_string());
                let _ = self.reload_notes();
            }

            // Count successful moves only when the source path disappeared.
            let moved_away = self.notes.iter().all(|n| n.path != *rel);
            if moved_away {
                moved += 1;
            } else {
                failed.push(rel.clone());
            }
        }
        self.notes_tree_cut_paths = failed;
        let target = if target_dir.is_empty() { "/" } else { target_dir.as_str() };
        if self.notes_tree_cut_paths.is_empty() {
            self.status = format!("moved {moved} note(s) to {target}");
        } else {
            self.status = match last_error {
                Some(err) => format!(
                    "moved {moved} note(s) to {target}, {} not moved: {err}",
                    self.notes_tree_cut_paths.len()
                ),
                None => format!(
                    "moved {moved} note(s) to {target}, {} not moved",
                    self.notes_tree_cut_paths.len()
                ),
            };
        }
        Ok(())
    }


    fn rebuild_notes_tree(&mut self) {
        let mut rows = Vec::new();
        if !self.current_notes_dir.is_empty() {
            rows.push(NotesTreeRow::ParentDir {
                path: Self::parent_dir_of(&self.current_notes_dir),
            });
        }


        let current = self.current_notes_dir.clone();
        for dir in self.indexed_dirs() {
            let parent = Self::parent_dir_of(&dir);
            if parent != current {
                continue;
            }
            let depth = 0;
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
                .path
                .parent()
                .and_then(|p| p.to_str())
                .map(str::to_string)
                .unwrap_or_default();
            if parent != current {
                continue;
            }
            let depth = 0;
            let name = note
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(ToString::to_string)
                .unwrap_or_else(|| note.display_path());
            rows.push(NotesTreeRow::Note {
                note_index: idx,
                depth,
                name,
            });
        }


        self.notes_tree_rows = rows;
        if self.selected_note >= self.notes_tree_rows.len() {
            self.selected_note =
self.notes_tree_rows.len().saturating_sub(1);
        }
        if let Some(anchor) = self.notes_tree_visual_anchor {
            if anchor >= self.notes_tree_rows.len() {
                self.notes_tree_visual_anchor =
self.notes_tree_rows.len().checked_sub(1);
            }
        }
    }


    fn selected_tree_note(&self) -> Option<&NoteMeta> {
        let row = self.notes_tree_rows.get(self.selected_note)?;
        match row {
            NotesTreeRow::ParentDir { .. } => None,
            NotesTreeRow::Note { note_index, .. } =>
self.notes.get(*note_index),
            NotesTreeRow::Dir { .. } => None,
        }
    }


    fn select_tree_row_by_path(&mut self, rel: &PathBuf) {
        let parent = rel
            .parent()
            .and_then(|p| p.to_str())
            .map(str::to_string)
            .unwrap_or_default();
        self.current_notes_dir = parent;
        self.rebuild_notes_tree();
        if let Some(pos) = self.notes_tree_rows.iter().position(|row|
match row {
            NotesTreeRow::ParentDir { .. } => false,
            NotesTreeRow::Note { note_index, .. } => self
                .notes
                .get(*note_index)
                .map(|n| n.path == *rel)
                .unwrap_or(false),
            NotesTreeRow::Dir { .. } => false,
        }) {
            self.selected_note = pos;
        }
    }


    fn activate_selected_note_tree_row(&mut self) ->
anyhow::Result<()> {
        let Some(row) =
self.notes_tree_rows.get(self.selected_note).cloned() else {
            return Ok(());
        };
        match row {
            NotesTreeRow::ParentDir { path } => {
                self.enter_notes_dir(&path);
                self.status = if path.is_empty() {
                    "entered /".to_string()
                } else {
                    format!("entered {path}/")
                };
            }
            NotesTreeRow::Dir { path, .. } => {
                self.enter_notes_dir(&path);
                self.status = format!("entered {path}/");
            }
            NotesTreeRow::Note { note_index, .. } => {
                if let Some(note) =
self.notes.get(note_index).cloned() {
                    self.open_note_by_path(&note.path)?;
                }
            }
        }
        Ok(())
    }


    fn expand_selected_dir(&mut self) {
        let Some(NotesTreeRow::Dir { path, .. }) =
self.notes_tree_rows.get(self.selected_note) else {
            return;
        };
        let path = path.clone();
        self.enter_notes_dir(&path);
        self.status = format!("entered {path}/");
    }


    fn collapse_selected_dir_or_parent(&mut self) {
        if self.current_notes_dir.is_empty() {
            return;
        }
        let parent = Self::parent_dir_of(&self.current_notes_dir);
        self.enter_notes_dir(&parent);
        self.status = if parent.is_empty() {
            "entered /".to_string()
        } else {
            format!("entered {parent}/")
        };
    }


    fn select_dir_row(&mut self, dir: &str) {
        let parent = Self::parent_dir_of(dir);
        self.current_notes_dir = parent;
        self.rebuild_notes_tree();
        if let Some(pos) = self.notes_tree_rows.iter().position(|row|
match row {
            NotesTreeRow::ParentDir { .. } => false,
            NotesTreeRow::Dir { path, .. } => path == dir,
            NotesTreeRow::Note { .. } => false,
        }) {
            self.selected_note = pos;
        }
    }
}
