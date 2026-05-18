impl App {
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

    fn create_new_note(
        &mut self,
        raw_dir: &str,
        raw_name: &str,
        raw_tags: &str,
    ) -> anyhow::Result<()> {
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
        let slug_input = monux_core::index::note_slug_with_dir(Some(raw_dir), normalized);
        if slug_input.is_empty() {
            self.status = "note path is invalid".to_string();
            return Ok(());
        }

        let note = self.index.create_note_with_tags(&slug_input, &tags)?;
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
}
