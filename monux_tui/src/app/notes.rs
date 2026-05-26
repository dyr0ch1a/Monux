use monux_core::index::{abs_note_path, note_path_with_dir,
resolve_note_path};


impl App {
    fn load_last_opened_path(&self) -> Option<PathBuf> {
        let content = std::fs::read_to_string(&self.state_path).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() == "last_opened_path" || key.trim() ==
"last_opened_slug" {
                let raw = value.trim().trim_matches('"');
                if !raw.is_empty() {
                    let rel = resolve_note_path(raw);
                    if !rel.as_os_str().is_empty() {
                        return Some(rel);
                    }
                }
            }
        }
        None
    }


    fn save_last_opened_path(&self, rel: &PathBuf) {
        if let Some(parent) = self.state_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let key = path_key(rel);
        let escaped = key.replace('"', "\\\"");
        let content = format!("last_opened_path = \"{escaped}\"\n");
        let _ = std::fs::write(&self.state_path, content);
    }


    fn clear_last_opened_path(&self) {
        let _ = std::fs::remove_file(&self.state_path);
    }


    fn selected_or_current_note(&self) -> Option<NoteMeta> {
        if self.focus == FocusPane::Notes {
            self.selected_tree_note().cloned()
        } else {
            None
        }
        .or_else(|| {
            self.current_note_rel
                .as_ref()
                .and_then(|rel| self.notes.iter().find(|n| n.path ==
*rel).cloned())
        })
    }


    fn rename_note_by_path(&mut self, note_path: &PathBuf, new_name:
&str) -> anyhow::Result<()> {
        let name = new_name.trim();
        if name.is_empty() {
            self.status = "new note name cannot be empty".to_string();
            return Ok(());
        }


        let Some(note) = self.notes.iter().find(|n| &n.path ==
note_path).cloned() else {
            self.status = "note not found".to_string();
            return Ok(());
        };


        if self.current_note_rel.as_ref() == Some(note_path) &&
self.dirty {
            self.status = "unsaved changes; save with :w
first".to_string();
            return Ok(());
        }


        let renamed_rel = if name.contains('/') || name.contains('\\')
{
            resolve_note_path(name)
        } else {
            let parent = note
                .path
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            note_path_with_dir(Some(parent), name)
        };
        if renamed_rel.as_os_str().is_empty() {
            self.status = "invalid note path".to_string();
            return Ok(());
        }
        if renamed_rel == note.path {
            self.status = "note already has this name".to_string();
            return Ok(());
        }
        let new_abs = abs_note_path(&self.notes_root, &renamed_rel);
        self.index.move_note(&note.path, &renamed_rel)?;


        let new_title = renamed_rel
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if !new_title.is_empty() {
            let current =
std::fs::read_to_string(&new_abs).unwrap_or_default();
            let updated = write_title_to_markdown(&current,
&new_title);
            if updated != current {
                std::fs::write(&new_abs, updated)?;
            }
        }


        let Some(renamed) = self.index.get_path(&renamed_rel)? else {
            self.status = "rename failed".to_string();
            return Ok(());
        };


        if let Some(mut buffer) = self.note_buffers.remove(note_path)
{
            buffer.path = new_abs.clone();
            self.note_buffers.insert(renamed.path.clone(), buffer);
        }
        if self.current_note_rel.as_ref() == Some(note_path) {
            self.current_note_rel = Some(renamed.path.clone());
            self.current_note_path = Some(new_abs);
        }


        self.reload_notes()?;
        self.select_tree_row_by_path(&renamed.path);
        self.status = format!(
            "renamed {} -> {}",
            note.display_path(),
            renamed.display_path()
        );
        Ok(())
    }


    fn move_selected_or_current_note_to_dir(&mut self, raw_dir: &str)
-> anyhow::Result<()> {
        let note = self.selected_or_current_note();


        let Some(note) = note else {
            self.status = "no note selected".to_string();
            return Ok(());
        };


        self.move_note_to_dir(note, raw_dir)
    }


    fn move_note_to_dir(&mut self, note: NoteMeta, raw_dir: &str) ->
anyhow::Result<()> {
        let dir_path =
monux_core::index::normalize_dir_path(raw_dir);
        let file_name = note
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let target_rel = note_path_with_dir(Some(&dir_path),
&file_name);
        if target_rel.as_os_str().is_empty() {
            self.status = "target directory is invalid".to_string();
            return Ok(());
        }
        if target_rel == note.path {
            self.status = "note is already in this
directory".to_string();
            return Ok(());
        }


        if self.current_note_rel.as_ref() == Some(&note.path) &&
self.dirty {
            self.status = "unsaved changes; save with :w
first".to_string();
            return Ok(());
        }


        let new_abs = abs_note_path(&self.notes_root, &target_rel);


        self.index.move_note(&note.path, &target_rel)?;
        let Some(renamed) = self.index.get_path(&target_rel)? else {
            self.status = "move failed".to_string();
            return Ok(());
        };


        if let Some(mut buffer) = self.note_buffers.remove(&note.path)
{
            buffer.path = new_abs.clone();
            self.note_buffers.insert(renamed.path.clone(), buffer);
        }


        if self.current_note_rel.as_ref() == Some(&note.path) {
            self.current_note_rel = Some(renamed.path.clone());
            self.current_note_path = Some(new_abs);
        }


        self.reload_notes()?;
        self.select_tree_row_by_path(&renamed.path);
        self.status = format!(
            "moved {} -> {}",
            note.display_path(),
            renamed.display_path()
        );
        Ok(())
    }


    fn open_selected_note(&mut self) -> anyhow::Result<()> {
        if self.notes_tree_rows.is_empty() {
            self.status = "notes index is empty".to_string();
            return Ok(());
        }


        let Some(note) = self.selected_tree_note().cloned() else {
            return Ok(());
        };


        self.open_note_by_path(&note.path)
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


    fn delete_note_by_title(&mut self, title: &str) ->
anyhow::Result<()> {
        let normalized = title.trim();
        let Some(note) = self
            .notes
            .iter()
            .find(|note| note.title == normalized)
            .cloned()
        else {
            self.status = format!("note with title '{}' not found",
normalized);
            return Ok(());
        };


        self.delete_note(note)
    }


    fn delete_note(&mut self, note: NoteMeta) -> anyhow::Result<()> {
        if self.dirty {
            self.status = "unsaved changes; save with :w
first".to_string();
            return Ok(());
        }


        self.index.delete_note(&note.path)?;
        self.reload_notes()?;


        self.drop_buffer_cache(&note.path);


        if self.current_note_rel.as_ref() == Some(&note.path) {
            if self.notes.is_empty() {
                self.reset_editor_to_empty();
            } else {
                self.open_selected_note()?;
            }
        }


        self.status = format!("deleted {}", note.display_path());
        Ok(())
    }


    fn open_selected_link(&mut self) -> anyhow::Result<()> {
        if self.links.is_empty() {
            return Ok(());
        }


        let Some(rel) = self.links.get(self.selected_link).cloned()
else {
            return Ok(());
        };


        self.open_note_by_path(&rel)
    }


    fn open_note_by_path(&mut self, rel: &PathBuf) ->
anyhow::Result<()> {
        self.open_note_by_path_force(rel, false)
    }


    fn open_note_by_path_force(&mut self, rel: &PathBuf, force: bool)
-> anyhow::Result<()> {
        if self.dirty && !force {
            self.status = "unsaved changes; use :w before switching or
:e!".to_string();
            return Ok(());
        }


        let normalized = resolve_note_path_ref(rel);
        if normalized.as_os_str().is_empty() {
            self.status = "invalid note path".to_string();
            return Ok(());
        }


        if !force && self.current_note_rel.as_ref() ==
Some(&normalized) {
            return Ok(());
        }


        let Some(note) = self.index.get_path(&normalized)? else {
            self.status = format!("note '{}' not found",
path_key(&normalized));
            return Ok(());
        };


        if let Some(current) = self.current_note_rel.clone() {
            if current != note.path {
                self.stash_current_buffer();
            }
        }


        if force {
            self.drop_buffer_cache(&note.path);
        }


        self.clear_editor_input_state();
        self.current_note_rel = Some(note.path.clone());
        self.save_last_opened_path(&note.path);


        if !force && self.restore_buffer_if_cached(&note.path) {
            self.editor_mode = EditorMode::Normal;
            self.ensure_has_line();
            self.select_tree_row_by_path(&note.path);
            self.status = format!("opened {} (buffer)",
note.display_path());
            self.on_editor_content_changed();
            return Ok(());
        }


        let abs = abs_note_path(&self.notes_root, &note.path);
        if !abs.exists() {
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::File::create(&abs)?;
        }


        let content = std::fs::read_to_string(&abs)?;
        self.editor_lines = split_lines(&content);
        self.ensure_has_line();
        self.current_note_path = Some(abs);
        self.editor_mode = EditorMode::Normal;
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.editor_scroll = 0;
        self.undo_stack.clear();
        self.dirty = false;


        self.select_tree_row_by_path(&note.path);


        self.status = format!("opened {}", note.display_path());
        self.on_editor_content_changed();
        Ok(())
    }


    fn save_current_note(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.current_note_path.clone() else {
            self.status = "no note opened; use :w <name>".to_string();
            return Ok(());
        };


        self.format_markdown_tables_in_place();
        let mut content = self.editor_lines.join("\n");
        if !content.is_empty() {
            content.push('\n');
        }


        std::fs::write(path, content.as_bytes())?;
        self.dirty = false;
        if let Some(rel) = self.current_note_rel.clone() {
            self.drop_buffer_cache(&rel);
        }
        self.status = "written".to_string();
        Ok(())
    }


    fn save_current_note_as(&mut self, raw_name: &str) ->
anyhow::Result<()> {
        let name = raw_name.trim();
        if name.is_empty() {
            self.status = "usage: :w <name>".to_string();
            return Ok(());
        }
        if self.current_note_rel.is_some() {
            self.status = "current note already has a name; use
:w".to_string();
            return Ok(());
        }


        let rel = note_path_with_dir(None, name);
        if rel.as_os_str().is_empty() {
            self.status = "note name is invalid".to_string();
            return Ok(());
        }


        let target_dir = rel
            .parent()
            .and_then(|p| p.to_str())
            .map(monux_core::index::normalize_dir_path)
            .unwrap_or_default();
        if !target_dir.is_empty() &&
!self.notes_root.join(&target_dir).is_dir() {
            self.status = format!("directory '{target_dir}' does not
exist; create it first");
            return Ok(());
        }


        std::fs::create_dir_all(&self.notes_root)?;
        self.storage.create_note(&rel, "")?;
        self.index.reindex_note(&rel)?;
        let Some(note) = self.index.get_path(&rel)? else {
            self.status = "create failed".to_string();
            return Ok(());
        };
        let abs = abs_note_path(&self.notes_root, &note.path);
        self.format_markdown_tables_in_place();
        let mut content = self.editor_lines.join("\n");
        if !content.is_empty() {
            content.push('\n');
        }
        std::fs::write(&abs, content.as_bytes())?;


        self.current_note_rel = Some(note.path.clone());
        self.current_note_path = Some(abs);
        self.dirty = false;
        self.reload_notes()?;
        self.select_tree_row_by_path(&note.path);
        self.status = format!("written {}", note.display_path());
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
                if let Some(rel) =
self.resolve_wikilink_target(target) {
                    links.insert(rel);
                }
                rest = &rest[end + 2..];
            }
        }


        self.links = links.into_iter().collect();
        if self.selected_link >= self.links.len() {
            self.selected_link = self.links.len().saturating_sub(1);
        }
    }


    fn resolve_wikilink_target(&self, target: &str) -> Option<PathBuf>
{
        let normalized = resolve_note_path(target);
        if normalized.as_os_str().is_empty() {
            return None;
        }


        if self.notes.iter().any(|note| note.path == normalized) {
            return Some(normalized);
        }


        let leaf = normalized
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let mut leaf_matches = self.notes.iter().filter(|note| {
            note.path
                .file_name()
                .and_then(|s| s.to_str())
                == Some(leaf)
        });
        let first_leaf = leaf_matches.next().map(|note|
note.path.clone());
        if let Some(rel) = first_leaf {
            if leaf_matches.next().is_none() {
                return Some(rel);
            }
        }


        let normalized_title = target.trim();
        let mut title_matches = self
            .notes
            .iter()
            .filter(|note| note.title.trim() == normalized_title);
        let first_title = title_matches.next().map(|note|
note.path.clone());
        if let Some(rel) = first_title {
            if title_matches.next().is_none() {
                return Some(rel);
            }
        }


        Some(normalized)
    }


    fn on_editor_content_changed(&mut self) {
        self.last_content_change_at = std::time::Instant::now();
        self.pending_links_refresh = true;
        self.pending_autosave = self.autosave_enabled && self.dirty;
    }


    fn autosave_current_note(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.current_note_path.clone() else {
            return Ok(());
        };
        self.format_markdown_tables_in_place();
        let mut content = self.editor_lines.join("\n");
        if !content.is_empty() {
            content.push('\n');
        }
        std::fs::write(path, content.as_bytes())?;
        self.dirty = false;
        if let Some(rel) = self.current_note_rel.clone() {
            self.drop_buffer_cache(&rel);
            self.last_autosave_rel = Some(rel);
            self.last_autosave_at = Some(std::time::Instant::now());
        }
        self.status = "autosaved".to_string();
        Ok(())
    }


    fn reload_notes_internal(&mut self, set_status: bool) -> anyhow::Result<()> {
        let selected_path = self.selected_tree_note().map(|n| n.path.clone());
        let paths = self.storage.list_note_paths()?;
        self.notes = paths
            .into_iter()
            .map(|path| {
                let title = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                NoteMeta { path, title }
            })
            .collect();
        self.rebuild_notes_tree();
        if let Some(rel) = selected_path {
            self.select_tree_row_by_path(&rel);
        }
        if set_status {
            self.status = format!("loaded {} notes", self.notes.len());
        }
        Ok(())
    }

    fn reload_notes(&mut self) -> anyhow::Result<()> {
        self.reload_notes_internal(true)?;
        Ok(())
    }


    fn sync_notes_index(&mut self, _dir: Option<&str>) ->
anyhow::Result<()> {
        let reindexed = self.index.reindex_all()?;
        let removed = self.index.prune_orphan_tags()?;
        let current_rel = self.current_note_rel.clone();
        self.reload_notes()?;
        if let Some(rel) = current_rel
            && self.notes.iter().all(|n| n.path != rel)
        {
            self.reset_editor_to_empty();
            self.status = "current note removed by sync".to_string();
        } else {
            self.status = format!("sync done: reindexed={reindexed},
orphan_tags_removed={removed}");
        }
        Ok(())
    }


    fn create_new_note(
        &mut self,
        raw_dir: &str,
        raw_name: &str,
        raw_tags: &str,
    ) -> anyhow::Result<()> {
        if self.dirty {
            self.status = "unsaved changes; save with :w
first".to_string();
            return Ok(());
        }


        let normalized = raw_name.trim();
        if normalized.is_empty() {
            self.status = "note name cannot be empty".to_string();
            return Ok(());
        }


        std::fs::create_dir_all(&self.notes_root)?;


        let tags = parse_tags_input(raw_tags);
        let rel = note_path_with_dir(Some(raw_dir), normalized);
        if rel.as_os_str().is_empty() {
            self.status = "note path is invalid".to_string();
            return Ok(());
        }
        let target_dir =
monux_core::index::normalize_dir_path(raw_dir);
        if !target_dir.is_empty() &&
!self.notes_root.join(&target_dir).is_dir() {
            self.status = format!("directory '{target_dir}' does not
exist; create it first");
            return Ok(());
        }


        let content =
monux_core::fsstorage::storage::tags_frontmatter(&tags);
        self.storage.create_note(&rel, &content)?;
        self.index.reindex_note(&rel)?;
        let Some(note) = self.index.get_path(&rel)? else {
            self.status = "create failed".to_string();
            return Ok(());
        };


        self.reload_notes()?;
        self.select_tree_row_by_path(&note.path);
        self.open_note_by_path_force(&note.path, true)?;
        if tags.is_empty() {
            self.status = format!("created {}", note.display_path());
        } else {
            self.status = format!(
                "created {} with #{}",
                note.display_path(),
                tags.join(" #")
            );
        }
        Ok(())
    }


    fn format_markdown_tables_in_place(&mut self) {
        let lines = align_markdown_table_lines(&self.editor_lines);
        if lines != self.editor_lines {
            self.editor_lines = lines;
            self.clamp_cursor();
            self.refresh_links();
        }
    }


}


fn resolve_note_path_ref(path: &PathBuf) -> PathBuf {
    monux_core::index::resolve_note_path_ref(path.as_path())
}


fn align_markdown_table_lines(lines: &[String]) -> Vec<String> {
    let mut out = lines.to_vec();
    let mut i = 0usize;


    while i + 1 < out.len() {
        if !is_table_row(&out[i]) || !is_table_separator_row(&out[i +
1]) {
            i += 1;
            continue;
        }


        let start = i;
        let mut end = i + 2;
        while end < out.len() && is_table_row(&out[end]) {
            end += 1;
        }


        align_table_block(&mut out[start..end]);
        i = end;
    }


    out
}


fn align_table_block(block: &mut [String]) {
    if block.is_empty() {
        return;
    }


    let mut parsed_rows: Vec<Vec<String>> =
Vec::with_capacity(block.len());
    let mut widths: Vec<usize> = Vec::new();


    for row in block.iter() {
        let cells = parse_table_cells(row);
        if widths.len() < cells.len() {
            widths.resize(cells.len(), 0);
        }
        for (idx, cell) in cells.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.chars().count());
        }
        parsed_rows.push(cells);
    }


    for (row_idx, row) in block.iter_mut().enumerate() {
        let original = row.as_str();
        let indent_len =
original.len().saturating_sub(original.trim_start().len());
        let indent = &original[..indent_len];
        let is_sep = row_idx == 1 && is_table_separator_row(original);
        let cells = &parsed_rows[row_idx];


        let mut rebuilt = String::new();
        rebuilt.push_str(indent);
        rebuilt.push('|');


        for (col_idx, width) in widths.iter().copied().enumerate() {
            rebuilt.push(' ');
            if is_sep {
                let sep = cells
                    .get(col_idx)
                    .map(|s| table_separator_cell(s, width))
                    .unwrap_or_else(|| "-".repeat(width.max(3)));
                rebuilt.push_str(&sep);
            } else {
                let cell = cells.get(col_idx).map(|s|
s.as_str()).unwrap_or("");
                rebuilt.push_str(cell);
                let pad = width.saturating_sub(cell.chars().count());
                if pad > 0 {
                    rebuilt.push_str(&" ".repeat(pad));
                }
            }
            rebuilt.push(' ');
            rebuilt.push('|');
        }


        *row = rebuilt;
    }
}


fn table_separator_cell(cell: &str, width: usize) -> String {
    let trimmed = cell.trim();
    let left = trimmed.starts_with(':');
    let right = trimmed.ends_with(':');
    let dash_len = width.max(3);
    let mut out = "-".repeat(dash_len);
    if left {
        out.replace_range(0..1, ":");
    }
    if right {
        let len = out.len();
        out.replace_range(len - 1..len, ":");
    }
    out
}


fn parse_table_cells(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let without_outer = trimmed.trim_matches('|');
    without_outer
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}


fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.contains('|')
}


fn is_table_separator_row(line: &str) -> bool {
    let cells = parse_table_cells(line);
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|cell| {
        let c = cell.trim();
        !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' ||
ch == ' ')
    })
}


fn write_title_to_markdown(content: &str, title: &str) -> String {
    let normalized = content.replace("\r\n", "\n");
    let safe_title = title.replace('"', "\\\"");
    if let Some((frontmatter, body)) = split_frontmatter(&normalized)
{
        let mut lines = frontmatter
            .lines()
            .map(ToString::to_string)
            .filter(|line| !line.trim().starts_with("title:"))
            .collect::<Vec<_>>();
        lines.insert(0, format!("title: \"{safe_title}\""));
        return format!("---\n{}\n---\n{}", lines.join("\n"), body);
    }


    format!("---\ntitle: \"{safe_title}\"\n---\n{normalized}")
}


fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let normalized = content.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    if lines.next()? != "---" {
        return None;
    }


    let mut frontmatter = Vec::new();
    let mut in_frontmatter = true;
    let mut body = Vec::new();
    for line in lines {
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            continue;
        }
        if in_frontmatter {
            frontmatter.push(line.to_string());
        } else {
            body.push(line.to_string());
        }
    }
    if in_frontmatter {
        return None;
    }


    let body_text = if body.is_empty() {
        String::new()
    } else {
        format!("{}\n", body.join("\n"))
    };
    Some((frontmatter.join("\n"), body_text))
}
