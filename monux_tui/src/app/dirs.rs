impl App {
    fn prompt_delete_selected_dir(&mut self) {
        let Some(NotesTreeRow::Dir { path, .. }) =
self.notes_tree_rows.get(self.selected_note) else {
            self.status = "select a directory to delete".to_string();
            return;
        };
        self.delete_dir_confirm_path = path.clone();
        self.delete_dir_confirm_popup = true;
        self.status = format!("delete directory {}/ ?", path);
    }


    fn delete_directory_confirmed(&mut self) -> anyhow::Result<()> {
        let path = std::mem::take(&mut self.delete_dir_confirm_path);
        self.delete_dir_confirm_popup = false;
        self.index.delete_dir(&path)?;
        self.expanded_dirs.remove(&path);
        self.rebuild_notes_tree();
        self.status = format!("deleted directory {path}/");
        Ok(())
    }


    fn handle_delete_dir_confirm_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') =>
{
                self.delete_dir_confirm_popup = false;
                self.delete_dir_confirm_path.clear();
                self.status = "delete directory
cancelled".to_string();
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')
=> {
                self.delete_directory_confirmed()?;
            }
            _ => {}
        }
        Ok(())
    }


    fn create_directory(&mut self, raw: &str) -> anyhow::Result<()> {
        let path = monux_core::index::normalize_dir_path(raw);
        if path.is_empty() {
            anyhow::bail!("directory path cannot be empty");
        }
        if self.notes_root.join(&path).exists() {
            anyhow::bail!("directory '{}' already exists", path);
        }
        if let Some(fs_path) = self.dir_fs_path(&path) {
            std::fs::create_dir_all(fs_path)?;
        }
        self.rebuild_notes_tree();
        self.select_dir_row(&path);
        self.status = format!("created directory {path}/");
        Ok(())
    }


    fn rename_directory_from_path(&mut self, old_raw: &str, new_raw:
&str) -> anyhow::Result<()> {
        let old_dir =
monux_core::index::normalize_dir_path(old_raw);
        let raw_new = new_raw.trim();
        if raw_new.is_empty() {
            self.status = "new directory name cannot be
empty".to_string();
            return Ok(());
        }
        let new_dir_candidate = if raw_new.contains('/') {
            raw_new.to_string()
        } else {
            let parent = old_dir
                .rsplit_once('/')
                .map(|(p, _)| p)
                .unwrap_or("");
            if parent.is_empty() {
                raw_new.to_string()
            } else {
                format!("{parent}/{raw_new}")
            }
        };
        let new_dir =
monux_core::index::normalize_dir_path(&new_dir_candidate);
        if old_dir.is_empty() || new_dir.is_empty() {
            self.status = "directory path cannot be
empty".to_string();
            return Ok(());
        }
        if old_dir == new_dir {
            self.status = "directory already has this
name".to_string();
            return Ok(());
        }
        if new_dir.starts_with(&(old_dir.clone() + "/")) {
            self.status = "cannot rename directory into its own
child".to_string();
            return Ok(());
        }


        let old_prefix = format!("{old_dir}/");
        let mut changed = Vec::new();
        for note in self.notes.clone() {
            let key = monux_core::index::path_key(&note.path);
            if key.starts_with(&old_prefix) {
                let rest = key.trim_start_matches(&old_prefix);
                let target = format!("{new_dir}/{rest}");
                let target_rel =
monux_core::index::normalize_note_path(&target);
                changed.push((note.path.clone(), target_rel));
            }
        }
        self.index.rename_dir(&old_dir, &new_dir)?;


        for (old_rel, new_rel) in &changed {
            if let Some(mut buffer) =
self.note_buffers.remove(old_rel) {
                buffer.path =
monux_core::index::abs_note_path(&self.notes_root, new_rel);
                self.note_buffers.insert(new_rel.clone(), buffer);
            }
        }


        if let Some(current) = self.current_note_rel.clone()
            && let Some((_, new_rel)) = changed.iter().find(|(old, _)|
old == &current)
        {
            self.current_note_rel = Some(new_rel.clone());
            self.current_note_path =

Some(monux_core::index::abs_note_path(&self.notes_root, new_rel));
        }


        if self.current_notes_dir == old_dir ||
self.current_notes_dir.starts_with(&(old_dir.clone() + "/")) {
            let rest = self
                .current_notes_dir
                .trim_start_matches(&(old_dir.clone() + "/"))
                .to_string();
            self.current_notes_dir = if self.current_notes_dir ==
old_dir {
                new_dir.clone()
            } else {
                format!("{new_dir}/{rest}")
            };
        }


        self.reload_notes()?;
        self.select_dir_row(&new_dir);
        self.status = format!("renamed directory {old_dir}/ ->
{new_dir}/");
        Ok(())
    }


    fn dir_fs_path(&self, dir: &str) -> Option<std::path::PathBuf> {
        if dir.is_empty() {
            Some(self.notes_root.clone())
        } else {
            Some(self.notes_root.join(dir))
        }
    }


    fn indexed_dirs(&self) -> Vec<String> {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&self.notes_root) {
            let Ok(entry) = entry else {
                continue;
            };
            if !entry.file_type().is_dir() {
                continue;
            }
            let path = entry.path();
            if path == self.notes_root {
                continue;
            }
            let Ok(rel) = path.strip_prefix(&self.notes_root) else {
                continue;
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let normalized =
monux_core::index::normalize_dir_path(&rel_str);
            if !normalized.is_empty() {
                out.push(normalized);
            }
        }
        out.sort();
        out.dedup();
        out
    }


    fn suggest_new_dir_parent(&self) -> String {
        match self.focus {
            FocusPane::Notes => {
                if let Some(NotesTreeRow::Dir { path, .. }) =
                    self.notes_tree_rows.get(self.selected_note)
                {
                    path.clone()
                } else if matches!(
                    self.notes_tree_rows.get(self.selected_note),
                    Some(NotesTreeRow::ParentDir { .. })
                ) {
                    self.current_notes_dir.clone()
                } else if let Some(note) = self.selected_tree_note() {
                    note.path
                        .parent()
                        .and_then(|p| p.to_str())
                        .map(str::to_string)
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            }
            FocusPane::Preview => self
                .current_note_rel
                .as_ref()
                .and_then(|rel| {
                    rel.parent()
                        .and_then(|p| p.to_str())
                        .map(str::to_string)
                })
                .unwrap_or_default(),
            FocusPane::Links => String::new(),
        }
    }


    fn handle_new_dir_popup_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.new_dir_popup = false;
                self.new_dir_input.clear();
                self.status = "new directory cancelled".to_string();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.new_dir_input);
                self.new_dir_popup = false;
                if let Err(err) = self.create_directory(&input) {
                    self.status = format!("new directory error:
{err}");
                }
            }
            KeyCode::Backspace => {
                self.new_dir_input.pop();
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.new_dir_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }
}


