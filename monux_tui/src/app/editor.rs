impl App {
    const MAX_EDITOR_LINE_CHARS: usize = 105;


    fn ensure_has_line(&mut self) {
        if self.editor_lines.is_empty() {
            self.editor_lines.push(String::new());
        }
    }


    fn push_undo_snapshot(&mut self) {
        let now = std::time::Instant::now();
        if let Some(last) = self.last_undo_snapshot_at
            && now.duration_since(last) < std::time::Duration::from_millis(250)
        {
            return;
        }
        self.last_undo_snapshot_at = Some(now);
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
        self.visual_line_mode = false;
        self.status = "-- VISUAL --".to_string();
    }


    fn enter_visual_line_mode(&mut self) {
        self.ensure_has_line();
        self.editor_mode = EditorMode::Visual;
        self.visual_anchor = Some((self.cursor_row, 0));
        self.visual_line_mode = true;
        self.status = "-- VISUAL LINE --".to_string();
    }


    fn exit_visual_mode(&mut self) {
        self.editor_mode = EditorMode::Normal;
        self.visual_anchor = None;
        self.visual_line_mode = false;
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
        self.cursor_row =
self.preview_prev_visible_row(self.cursor_row);
        self.clamp_cursor_col();
    }


    fn move_cursor_down(&mut self) {
        self.cursor_row =
self.preview_next_visible_row(self.cursor_row);
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
            let chars: Vec<char> =
self.editor_lines[row].chars().collect();
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


    fn move_word_forward(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }


        let mut row = self
            .cursor_row
            .min(self.editor_lines.len().saturating_sub(1));
        let mut col = self.cursor_col;


        loop {
            let chars: Vec<char> =
self.editor_lines[row].chars().collect();
            if chars.is_empty() {
                if row + 1 >= self.editor_lines.len() {
                    self.cursor_row = row;
                    self.cursor_col = 0;
                    return;
                }
                self.cursor_row = row + 1;
                self.cursor_col = 0;
                self.clamp_cursor();
                return;
            }


            if col >= chars.len() {
                if row + 1 >= self.editor_lines.len() {
                    self.cursor_row = row;
                    self.cursor_col = chars.len().saturating_sub(1);
                    self.clamp_cursor();
                    return;
                }
                self.cursor_row = row + 1;
                self.cursor_col = 0;
                self.clamp_cursor();
                return;
            }


            let mut i = col.min(chars.len().saturating_sub(1));


            if is_word_char(chars[i]) {
                while i + 1 < chars.len() && is_word_char(chars[i +
1]) {
                    i += 1;
                }
                if i + 1 < chars.len() {
                    i += 1;
                } else if row + 1 < self.editor_lines.len() {
                    self.cursor_row = row + 1;
                    self.cursor_col = 0;
                    self.clamp_cursor();
                    return;
                }
            }


            while i < chars.len() && !is_word_char(chars[i]) {
                i += 1;
            }


            if i < chars.len() {
                self.cursor_row = row;
                self.cursor_col = i;
                self.clamp_cursor();
                return;
            }


            if row + 1 >= self.editor_lines.len() {
                self.cursor_row = row;
                self.cursor_col = chars.len().saturating_sub(1);
                self.clamp_cursor();
                return;
            }


            row += 1;
            col = 0;
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
            let chars: Vec<char> =
self.editor_lines[row].chars().collect();
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


            if is_word_char(chars[i]) {
                while i + 1 < chars.len() && is_word_char(chars[i +
1]) {
                    i += 1;
                }


                if i > col {
                    self.cursor_row = row;
                    self.cursor_col = i;
                    self.clamp_cursor();
                    return;
                }


                if i + 1 < chars.len() {
                    i += 1;
                } else if row + 1 < self.editor_lines.len() {
                    if self.cursor_row == row && self.cursor_col >= i
{
                        row += 1;
                        col = 0;
                        continue;
                    }
                    self.cursor_row = row;
                    self.cursor_col = i;
                    self.clamp_cursor();
                    return;
                } else {
                    self.cursor_row = row;
                    self.cursor_col = i;
                    self.clamp_cursor();
                    return;
                }
            }


            while i < chars.len() && !is_word_char(chars[i]) {
                i += 1;
            }


            if i >= chars.len() {
                let line_end = chars.len().saturating_sub(1);
                if row + 1 < self.editor_lines.len()
                    && self.cursor_row == row
                    && self.cursor_col >= line_end
                {
                    row += 1;
                    col = 0;
                    continue;
                }
                self.cursor_row = row;
                self.cursor_col = line_end;
                self.clamp_cursor();
                return;
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
        let current_len = self.current_line_len_chars();
        let insert_width = if ch == '[' { 2 } else { 1 };
        if current_len + insert_width > Self::MAX_EDITOR_LINE_CHARS {
            self.split_line_at_cursor();
            self.insert_char(ch);
            return;
        }
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


        let continuation = list_continuation_prefix(&self.editor_lines[row], col);
        let idx = char_to_byte_idx(&self.editor_lines[row], col);
        let tail = self.editor_lines[row].split_off(idx);
        self.editor_lines.insert(row + 1, format!("{continuation}{tail}"));
        self.cursor_row += 1;
        self.cursor_col = continuation.chars().count();
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
            self.cursor_row =
self.editor_lines.len().saturating_sub(1);
        }


        self.clamp_cursor_col();
        self.dirty = true;
        self.on_editor_content_changed();
        self.status = "deleted line".to_string();
    }


    fn delete_visual_selection(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) =
self.normalized_visual_bounds()
        else {
            self.exit_visual_mode();
            return;
        };


        self.push_undo_snapshot();


        if self.visual_line_mode {
            self.editor_lines.drain(start_row..=end_row);
            self.ensure_has_line();
            self.cursor_row =
start_row.min(self.editor_lines.len().saturating_sub(1));
            self.cursor_col = 0;
            self.clamp_cursor();
            self.dirty = true;
            self.on_editor_content_changed();
            self.exit_visual_mode();
            self.status = "deleted lines".to_string();
            return;
        }


        if start_row == end_row {
            if let Some(line) = self.editor_lines.get_mut(start_row) {
                let line_len = line.chars().count();
                let start = start_col.min(line_len);
                let end_exclusive =
end_col.saturating_add(1).min(line_len);
                if start < end_exclusive {
                    let start_byte = char_to_byte_idx(line, start);
                    let end_byte = char_to_byte_idx(line,
end_exclusive);
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
                let start =
end_col.saturating_add(1).min(end_line_len);
                let start_byte = char_to_byte_idx(&end_line, start);
                end_line[start_byte..].to_string()
            };


            self.editor_lines[start_row] =
format!("{prefix}{suffix}");
            self.editor_lines.drain((start_row + 1)..=end_row);
        }


        self.ensure_has_line();
        self.cursor_row =
start_row.min(self.editor_lines.len().saturating_sub(1));
        self.cursor_col = start_col;
        self.clamp_cursor();
        self.dirty = true;
        self.on_editor_content_changed();
        self.exit_visual_mode();
        self.status = "deleted selection".to_string();
    }


    fn delete_motion_range(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        inclusive_end: bool,
    ) {
        self.ensure_has_line();
        let (mut start_row, mut start_col) = start;
        let (mut end_row, mut end_col) = end;
        if (start_row, start_col) > (end_row, end_col) {
            std::mem::swap(&mut start_row, &mut end_row);
            std::mem::swap(&mut start_col, &mut end_col);
        }


        if !inclusive_end {
            if start_row == end_row && start_col == end_col {
                self.status = "nothing to delete".to_string();
                self.clamp_cursor();
                return;
            }
            if end_col > 0 {
                end_col -= 1;
            } else if end_row > start_row {
                end_row -= 1;
                end_col = self.editor_lines[end_row].chars().count();
                end_col = end_col.saturating_sub(1);
            } else {
                self.status = "nothing to delete".to_string();
                self.clamp_cursor();
                return;
            }
        }


        self.visual_anchor = Some((start_row, start_col));
        self.cursor_row = end_row;
        self.cursor_col = end_col;
        self.editor_mode = EditorMode::Visual;
        self.delete_visual_selection();
        self.editor_mode = EditorMode::Normal;
        self.visual_anchor = None;
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
            let insert_at = (self.cursor_row +
1).min(self.editor_lines.len());
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
            self.editor_lines[row] = format!("{prefix}{}{suffix}",
parts[0]);
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
        let ((start_row, start_col), (end_row, end_col)) =
self.normalized_visual_bounds()?;
        if self.visual_line_mode {
            let mut out = String::new();
            for row in start_row..=end_row {
                let line = self.editor_lines.get(row)?;
                out.push_str(line);
                if row < end_row {
                    out.push('\n');
                }
            }
            return Some(YankRegister {
                text: out,
                linewise: true,
            });
        }
        let mut out = String::new();


        if start_row == end_row {
            let line = self.editor_lines.get(start_row)?;
            let line_len = line.chars().count();
            let start = start_col.min(line_len);
            let end_exclusive =
end_col.saturating_add(1).min(line_len);
            let start_byte = char_to_byte_idx(line, start);
            let end_byte = char_to_byte_idx(line, end_exclusive);
            out.push_str(&line[start_byte..end_byte]);
        } else {
            let start_line = self.editor_lines.get(start_row)?;
            let start_line_len = start_line.chars().count();
            let start_byte = char_to_byte_idx(start_line,
start_col.min(start_line_len));
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
            let end_exclusive =
end_col.saturating_add(1).min(end_line_len);
            let end_byte = char_to_byte_idx(end_line, end_exclusive);
            out.push_str(&end_line[..end_byte]);
        }


        Some(YankRegister {
            text: out,
            linewise: false,
        })
    }


    fn normalized_visual_bounds(&self) -> Option<((usize, usize),
(usize, usize))> {
        if self.editor_mode != EditorMode::Visual {
            return None;
        }


        let anchor = self.visual_anchor?;
        let cursor = (self.cursor_row, self.cursor_col);


        if self.visual_line_mode {
            let (start_row, end_row) = if anchor.0 <= cursor.0 {
                (anchor.0, cursor.0)
            } else {
                (cursor.0, anchor.0)
            };
            let end_col = self
                .editor_lines
                .get(end_row)
                .map(|line| line.chars().count().saturating_sub(1))
                .unwrap_or(0);
            Some(((start_row, 0), (end_row, end_col)))
        } else if anchor <= cursor {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }
}

fn list_continuation_prefix(line: &str, col: usize) -> String {
    let split_idx = char_to_byte_idx(line, col);
    let left = &line[..split_idx];
    let trimmed = left.trim_start_matches([' ', '\t']);
    let indent_len = left.len().saturating_sub(trimmed.len());
    let indent = &left[..indent_len];

    // Continue unordered markers: "-", "*", "+".
    for marker in ["- ", "* ", "+ "] {
        if trimmed.starts_with(marker) {
            let body = &trimmed[marker.len()..];
            if body.starts_with("[ ] ") || body.starts_with("[x] ") || body.starts_with("[X] ") {
                if body[4..].trim().is_empty() {
                    return String::new();
                }
                return format!("{indent}{marker}[ ] ");
            }
            if trimmed[marker.len()..].trim().is_empty() {
                return String::new();
            }
            return format!("{indent}{marker}");
        }
    }

    // Continue ordered markers: "1. " and "1) ".
    let bytes = trimmed.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i + 1 < bytes.len() && (bytes[i] == b'.' || bytes[i] == b')') && bytes[i + 1] == b' ' {
        let marker_char = bytes[i] as char;
        let number = trimmed[..i].parse::<usize>().unwrap_or(1);
        if trimmed[i + 2..].trim().is_empty() {
            return String::new();
        }
        return format!("{indent}{}{marker_char} ", number + 1);
    }

    String::new()
}
