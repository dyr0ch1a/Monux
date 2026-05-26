impl App {
    pub fn preview_source_lines(&self) -> &[String] {
        &self.editor_lines
    }


    pub fn preview_is_heading_row(&self, row: usize) -> bool {
        self.preview_heading_level(row).is_some()
    }


    pub fn preview_heading_level(&self, row: usize) -> Option<usize> {
        let line = self.editor_lines.get(row)?;
        let trimmed = line.trim_start();
        let hashes = trimmed.chars().take_while(|ch| *ch ==
'#').count();
        if (1..=6).contains(&hashes) &&
trimmed.chars().nth(hashes).is_some_and(|ch| ch.is_whitespace()) {
            Some(hashes)
        } else {
            None
        }
    }


    fn preview_heading_section_end(&self, heading_row: usize) -> usize
{
        let Some(level) = self.preview_heading_level(heading_row) else
{
            return heading_row.saturating_add(1);
        };


        for row in heading_row + 1..self.editor_lines.len() {
            if let Some(next_level) = self.preview_heading_level(row)
{
                if next_level <= level {
                    return row;
                }
            }
        }


        self.editor_lines.len()
    }


    fn preview_heading_row_for_row(&self, row: usize) -> Option<usize>
{
        if self.preview_is_heading_row(row) {
            return Some(row);
        }


        let mut candidate = row;
        while candidate > 0 {
            candidate -= 1;
            let Some(level) = self.preview_heading_level(candidate)
else {
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
            .any(|heading_row| row > heading_row && row <
self.preview_heading_section_end(heading_row))
    }


    pub fn preview_heading_is_collapsed(&self, row: usize) -> bool {
        self.collapsed_headings.contains(&row)
    }


    pub fn preview_visible_row_for_source_row(&self, row: usize) ->
usize {
        if self.preview_is_row_hidden(row) {
            self.preview_heading_row_for_row(row).unwrap_or(row)
        } else {
            row
        }
    }


    pub fn preview_screen_row_for_source_row(&self, scroll: usize,
row: usize) -> usize {
        if self.editor_lines.is_empty() || scroll >=
self.editor_lines.len() {
            return 0;
        }


        let target = self.preview_visible_row_for_source_row(row);
        if target < scroll {
            return 0;
        }


        let mut visible = 0usize;
        for idx in
scroll..=target.min(self.editor_lines.len().saturating_sub(1)) {
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


        let visible_row =
self.preview_visible_row_for_source_row(row);
        if let Some(heading_row) =
self.preview_heading_row_for_row(visible_row)
            && self.collapsed_headings.contains(&heading_row)
        {
            let next = self.preview_heading_section_end(heading_row);
            if next < self.editor_lines.len() {
                return next;
            }
            return visible_row;
        }


        let mut next = visible_row.saturating_add(1);
        while next < self.editor_lines.len() &&
self.preview_is_row_hidden(next) {
            next += 1;
        }
        if next < self.editor_lines.len() { next } else { visible_row
}
    }


    pub fn preview_prev_visible_row(&self, row: usize) -> usize {
        if self.editor_lines.is_empty() {
            return 0;
        }


        let visible_row =
self.preview_visible_row_for_source_row(row);
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
        let row =
self.preview_visible_row_for_source_row(self.cursor_row);
        let Some(heading_row) = self.preview_heading_row_for_row(row)
else {
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
        if self.focus == FocusPane::Notes &&
self.notes_tree_visual_mode {
            return "NOTES-VISUAL";
        }
        match self.editor_mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual => "VISUAL",
        }
    }


    pub fn is_visual_mode(&self) -> bool {
        self.editor_mode == EditorMode::Visual
    }


    pub fn cursor_style_name(&self) -> &'static str {
        if self.command_mode
            || self.search_mode
            || self.new_note_popup
            || self.global_search_popup
            || self.new_dir_popup
        {
            return "bar";
        }


        if self.focus == FocusPane::Notes &&
self.notes_tree_visual_mode {
            return "underscore";
        }


        match self.editor_mode {
            EditorMode::Insert => "bar",
            EditorMode::Visual => "underscore",
            EditorMode::Normal => "block",
        }
    }


    pub fn notes_tree_labels(&self) -> Vec<String> {
        self.notes_tree_rows
            .iter()
            .enumerate()
            .map(|(row_idx, row)| match row {
                NotesTreeRow::ParentDir { .. } => "..".to_string(),
                NotesTreeRow::Dir { path, depth, name } => {
                    let _ = name;
                    let label = display_name_from_note_path(path);
                    format!("  {}{label}/", "  ".repeat(*depth))
                }
                NotesTreeRow::Note { note_index, depth, name } => {
                    let note = &self.notes[*note_index];
                    let in_visual = self
                        .notes_tree_selection_bounds()
                        .map(|(s, e)| row_idx >= s && row_idx <= e)
                        .unwrap_or(false);
                    let marker = if in_visual {
                        ">"
                    } else if
self.notes_tree_cut_paths.iter().any(|path| path == &note.path) {
                        "x"
                    } else if self.current_note_rel.as_ref() ==
Some(&note.path) {
                        "*"
                    } else {
                        " "
                    };
                    let raw_label = if note.title.trim().is_empty() {
                        name.as_str()
                    } else {
                        note.title.as_str()
                    };
                    let label = raw_label
                        .rsplit('/')
                        .next()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or(raw_label);
                    format!("{marker} {}{label}", "  ".repeat(*depth))
                }
            })
            .collect()
    }


    pub fn notes_tree_len(&self) -> usize {
        self.notes_tree_rows.len()
    }


    pub fn notes_tree_title(&self) -> String {
        if self.current_notes_dir.is_empty() {
            "Notes: /".to_string()
        } else {
            format!("Notes: /{}", self.current_notes_dir)
        }
    }


    pub fn link_labels(&self) -> Vec<String> {
        self.links
            .iter()
            .map(|path| {
                self.notes
                    .iter()
                    .find(|note| note.path == *path)
                    .map(|note| {
                        let title = note.title.trim();
                        if title.is_empty() {

display_name_from_note_path(&path_key(&note.path))
                        } else {
                            title.to_string()
                        }
                    })
                    .unwrap_or_else(||
display_name_from_note_path(&path_key(path)))
            })
            .collect()
    }


    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }


    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }


    pub fn visual_selection_for_row(&self, row: usize, line_len:
usize) -> Option<(usize, usize)> {
        let ((start_row, start_col), (end_row, end_col)) =
self.normalized_visual_bounds()?;
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


        let cursor_row =
self.preview_visible_row_for_source_row(self.cursor_row);
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


