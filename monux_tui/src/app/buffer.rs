#[derive(Clone)]
struct NoteBuffer {
    lines: Vec<String>,
    path: PathBuf,
    cursor_row: usize,
    cursor_col: usize,
    scroll: u16,
    dirty: bool,
    undo_stack: Vec<UndoState>,
}


impl App {
    const MAX_NOTE_BUFFER_CACHE: usize = 24;

    fn touch_note_buffer_lru(&mut self, rel: &PathBuf) {
        if let Some(pos) = self.note_buffer_lru.iter().position(|p| p == rel) {
            self.note_buffer_lru.remove(pos);
        }
        self.note_buffer_lru.push_back(rel.clone());
    }

    fn evict_note_buffer_cache_if_needed(&mut self) {
        while self.note_buffers.len() > Self::MAX_NOTE_BUFFER_CACHE {
            let Some(oldest) = self.note_buffer_lru.pop_front() else {
                break;
            };
            self.note_buffers.remove(&oldest);
        }
    }

    fn stash_current_buffer(&mut self) {
        let Some(rel) = self.current_note_rel.clone() else {
            return;
        };
        let Some(path) = self.current_note_path.clone() else {
            return;
        };


        self.note_buffers.insert(
            rel.clone(),
            NoteBuffer {
                lines: self.editor_lines.clone(),
                path,
                cursor_row: self.cursor_row,
                cursor_col: self.cursor_col,
                scroll: self.editor_scroll,
                dirty: self.dirty,
                undo_stack: self.undo_stack.clone(),
            },
        );
        self.touch_note_buffer_lru(&rel);
        self.evict_note_buffer_cache_if_needed();
    }


    fn restore_buffer_if_cached(&mut self, rel: &PathBuf) -> bool {
        let Some(buffer) = self.note_buffers.remove(rel) else {
            return false;
        };
        if let Some(pos) = self.note_buffer_lru.iter().position(|p| p == rel) {
            self.note_buffer_lru.remove(pos);
        }


        self.editor_lines = buffer.lines;
        self.current_note_path = Some(buffer.path);
        self.cursor_row = buffer.cursor_row;
        self.cursor_col = buffer.cursor_col;
        self.editor_scroll = buffer.scroll;
        self.dirty = buffer.dirty;
        self.undo_stack = buffer.undo_stack;
        true
    }


    fn drop_buffer_cache(&mut self, rel: &PathBuf) {
        self.note_buffers.remove(rel);
        if let Some(pos) = self.note_buffer_lru.iter().position(|p| p == rel) {
            self.note_buffer_lru.remove(pos);
        }
    }


    fn clear_editor_input_state(&mut self) {
        self.pending_normal_op = None;
        self.pending_leader = false;
        self.motion_count = None;
        self.visual_anchor = None;
    }


    fn reset_editor_to_empty(&mut self) {
        self.editor_lines = vec![String::new()];
        self.current_note_rel = None;
        self.current_note_path = None;
        self.clear_last_opened_path();
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.editor_scroll = 0;
        self.dirty = false;
        self.undo_stack.clear();
        self.clear_editor_input_state();
        self.editor_mode = EditorMode::Normal;
        self.links.clear();
        self.selected_link = 0;
    }


    fn take_motion_count(&mut self) -> usize {
        self.motion_count.take().unwrap_or(1).max(1)
    }


    fn push_motion_digit(&mut self, digit: u8) -> bool {
        let value = (self.motion_count.unwrap_or(0) as u32) * 10 +
digit as u32;
        if value == 0 {
            return false;
        }
        self.motion_count = Some(value as usize);
        self.pending_normal_op = None;
        self.status = format!("count: {}",
self.motion_count.unwrap_or(1));
        true
    }


    fn repeat_motion<F>(&mut self, count: usize, mut action: F)
    where
        F: FnMut(&mut Self),
    {
        for _ in 0..count {
            action(self);
        }
    }
}

