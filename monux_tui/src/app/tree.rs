impl App {
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
}
