use crossterm::cursor::MoveTo;
use std::io::Write;

impl App {
    pub fn resolve_preview_image_path(&self, raw: &str) -> Option<PathBuf> {
        let path = PathBuf::from(raw.trim());
        if path.as_os_str().is_empty() {
            return None;
        }

        if path.is_absolute() {
            return Some(path);
        }

        if let Some(current) = self.current_note_path.as_ref()
            && let Some(parent) = current.parent()
        {
            let candidate = parent.join(&path);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        let candidate = self.notes_root.join(&path);
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    }

    pub fn preview_source_lines(&self) -> &[String] {
        &self.editor_lines
    }

    pub fn preview_is_heading_row(&self, row: usize) -> bool {
        self.preview_heading_level(row).is_some()
    }

    pub fn preview_heading_level(&self, row: usize) -> Option<usize> {
        let line = self.editor_lines.get(row)?;
        let trimmed = line.trim_start();
        let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
        if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes).is_some_and(|ch| ch.is_whitespace()) {
            Some(hashes)
        } else {
            None
        }
    }

    fn preview_heading_section_end(&self, heading_row: usize) -> usize {
        let Some(level) = self.preview_heading_level(heading_row) else {
            return heading_row.saturating_add(1);
        };

        for row in heading_row + 1..self.editor_lines.len() {
            if let Some(next_level) = self.preview_heading_level(row) {
                if next_level <= level {
                    return row;
                }
            }
        }

        self.editor_lines.len()
    }

    fn preview_heading_row_for_row(&self, row: usize) -> Option<usize> {
        if self.preview_is_heading_row(row) {
            return Some(row);
        }

        let mut candidate = row;
        while candidate > 0 {
            candidate -= 1;
            let Some(level) = self.preview_heading_level(candidate) else {
                continue;
            };
            if row < self.preview_heading_section_end(candidate) {
                return Some(candidate);
            }
            if level == 1 {
                break;
            }
        }

        None
    }

    pub fn preview_is_row_hidden(&self, row: usize) -> bool {
        self.collapsed_headings
            .iter()
            .copied()
            .any(|heading_row| row > heading_row && row < self.preview_heading_section_end(heading_row))
    }

    pub fn preview_heading_is_collapsed(&self, row: usize) -> bool {
        self.collapsed_headings.contains(&row)
    }

    pub fn preview_visible_row_for_source_row(&self, row: usize) -> usize {
        if self.preview_is_row_hidden(row) {
            self.preview_heading_row_for_row(row).unwrap_or(row)
        } else {
            row
        }
    }

    pub fn preview_screen_row_for_source_row(&self, scroll: usize, row: usize) -> usize {
        if self.editor_lines.is_empty() || scroll >= self.editor_lines.len() {
            return 0;
        }

        let target = self.preview_visible_row_for_source_row(row);
        if target < scroll {
            return 0;
        }

        let mut visible = 0usize;
        for idx in scroll..=target.min(self.editor_lines.len().saturating_sub(1)) {
            if !self.preview_is_row_hidden(idx) {
                visible += 1;
            }
        }

        visible.saturating_sub(1)
    }

    pub fn preview_next_visible_row(&self, row: usize) -> usize {
        if self.editor_lines.is_empty() {
            return 0;
        }

        let visible_row = self.preview_visible_row_for_source_row(row);
        if let Some(heading_row) = self.preview_heading_row_for_row(visible_row)
            && self.collapsed_headings.contains(&heading_row)
        {
            let next = self.preview_heading_section_end(heading_row);
            if next < self.editor_lines.len() {
                return next;
            }
            return visible_row;
        }

        let mut next = visible_row.saturating_add(1);
        while next < self.editor_lines.len() && self.preview_is_row_hidden(next) {
            next += 1;
        }
        if next < self.editor_lines.len() { next } else { visible_row }
    }

    pub fn preview_prev_visible_row(&self, row: usize) -> usize {
        if self.editor_lines.is_empty() {
            return 0;
        }

        let visible_row = self.preview_visible_row_for_source_row(row);
        if visible_row == 0 {
            return 0;
        }

        let mut prev = visible_row - 1;
        while self.preview_is_row_hidden(prev) {
            if prev == 0 {
                return 0;
            }
            prev -= 1;
        }
        prev
    }

    pub fn toggle_current_heading_fold(&mut self) -> bool {
        let row = self.preview_visible_row_for_source_row(self.cursor_row);
        let Some(heading_row) = self.preview_heading_row_for_row(row) else {
            self.status = "cursor is not on a heading".to_string();
            return false;
        };

        if !self.collapsed_headings.insert(heading_row) {
            self.collapsed_headings.remove(&heading_row);
            self.status = "heading expanded".to_string();
        } else {
            self.status = "heading collapsed".to_string();
        }

        self.clamp_cursor();
        true
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

        let cursor_row = self.preview_visible_row_for_source_row(self.cursor_row);
        let top = self.editor_scroll as usize;
        let bottom = top + view_height.saturating_sub(1);

        if cursor_row < top {
            self.editor_scroll = cursor_row as u16;
        } else if cursor_row > bottom {
            let new_top = cursor_row
                .saturating_sub(view_height.saturating_sub(1));
            self.editor_scroll = new_top as u16;
        }
    }

}

fn percent_encode_path(path: &PathBuf) -> String {
    let mut out = String::new();
    for byte in path.to_string_lossy().as_bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'/'
            | b':' => out.push(*byte as char),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", byte));
            }
        }
    }
    out
}
