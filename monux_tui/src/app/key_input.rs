impl App {
    pub fn on_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.new_note_popup {
            return self.handle_new_note_popup_input(key);
        }
        if self.help_popup {
            return self.handle_help_popup_input(key);
        }
        if self.command_mode {
            return self.handle_command_input(key);
        }

        if self.editor_mode == EditorMode::Normal && self.handle_leader_key(key)? {
            return Ok(());
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('s')) {
            self.save_current_note()?;
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('n')) {
            self.new_note_popup = true;
            self.new_note_dir_input = self.suggest_new_note_dir();
            self.new_note_input.clear();
            self.new_note_tags_input.clear();
            self.new_note_field = NewNoteField::Name;
            self.status = "new note".to_string();
            return Ok(());
        }
        if self.focus == FocusPane::Preview && self.editor_mode == EditorMode::Insert {
            self.handle_insert_mode_key(key);
            return Ok(());
        }

        if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('?'))
        {
            self.help_popup = true;
            self.pending_leader = false;
            return Ok(());
        }

        if self.focus == FocusPane::Preview {
            if self.editor_mode == EditorMode::Visual {
                self.handle_visual_mode_key(key)?;
                return Ok(());
            }

            if self.handle_preview_normal_key(key)? {
                return Ok(());
            }
        }

        match key.code {
            KeyCode::Char('q') => {
                if self.dirty {
                    self.status = "unsaved changes; use :q! to discard or :w".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Tab => {
                self.cycle_focus(true);
            }
            KeyCode::BackTab => {
                self.cycle_focus(false);
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                self.pending_leader = false;
            }
            KeyCode::Enter => match self.focus {
                FocusPane::Notes => self.activate_selected_note_tree_row()?,
                FocusPane::Links => self.open_selected_link()?,
                FocusPane::Preview => {}
            },
            KeyCode::Right | KeyCode::Char('l') => {
                if self.focus == FocusPane::Notes {
                    self.expand_selected_dir();
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.focus == FocusPane::Notes {
                    self.collapse_selected_dir_or_parent();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Char('D') => {
                if self.focus == FocusPane::Notes {
                    self.delete_selected_note()?;
                }
            }
            KeyCode::Char('r') => {
                self.reload_notes()?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_leader_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            self.pending_leader = false;
            return Ok(false);
        }

        if self.pending_leader {
            self.pending_leader = false;
            match key.code {
                KeyCode::Char('e') => {
                    self.toggle_notes_panel();
                    return Ok(true);
                }
                KeyCode::Char('b') => {
                    self.toggle_links_panel();
                    return Ok(true);
                }
                KeyCode::Char(' ') => {
                    return Ok(true);
                }
                _ => {
                    return Ok(false);
                }
            }
        }

        if matches!(key.code, KeyCode::Char(' ')) {
            self.pending_leader = true;
            self.status = "leader: <space>e notes, <space>b links".to_string();
            return Ok(true);
        }

        Ok(false)
    }

    fn toggle_notes_panel(&mut self) {
        self.show_notes_panel = !self.show_notes_panel;
        if self.show_notes_panel {
            self.focus = FocusPane::Notes;
            self.status = "notes pane shown".to_string();
        } else {
            if self.focus == FocusPane::Notes {
                self.focus = FocusPane::Preview;
            }
            self.status = "notes pane hidden".to_string();
        }
        self.ensure_focus_visible();
    }

    fn toggle_links_panel(&mut self) {
        self.show_links_panel = !self.show_links_panel;
        if self.show_links_panel {
            self.focus = FocusPane::Links;
            self.status = "links pane shown".to_string();
        } else {
            if self.focus == FocusPane::Links {
                self.focus = FocusPane::Preview;
            }
            self.status = "links pane hidden".to_string();
        }
        self.ensure_focus_visible();
    }

    fn cycle_focus(&mut self, forward: bool) {
        let panes = self.visible_panes();
        if panes.is_empty() {
            self.focus = FocusPane::Preview;
            return;
        }

        let current = panes
            .iter()
            .position(|pane| *pane == self.focus)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % panes.len()
        } else if current == 0 {
            panes.len() - 1
        } else {
            current - 1
        };

        self.focus = panes[next];
    }

    fn visible_panes(&self) -> Vec<FocusPane> {
        let mut panes = Vec::with_capacity(3);
        if self.show_notes_panel {
            panes.push(FocusPane::Notes);
        }
        panes.push(FocusPane::Preview);
        if self.show_links_panel {
            panes.push(FocusPane::Links);
        }
        panes
    }

    fn ensure_focus_visible(&mut self) {
        if (self.focus == FocusPane::Notes && !self.show_notes_panel)
            || (self.focus == FocusPane::Links && !self.show_links_panel)
        {
            self.focus = FocusPane::Preview;
        }
    }

    fn handle_command_input(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_input.clear();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.command_input);
                self.command_mode = false;
                self.execute_command(input.trim())?;
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_new_note_popup_input(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.new_note_popup = false;
                self.new_note_dir_input.clear();
                self.new_note_input.clear();
                self.new_note_tags_input.clear();
                self.new_note_field = NewNoteField::Name;
                self.status = "new note cancelled".to_string();
            }
            KeyCode::Enter => {
                let dir = std::mem::take(&mut self.new_note_dir_input);
                let name = std::mem::take(&mut self.new_note_input);
                let tags = std::mem::take(&mut self.new_note_tags_input);
                self.new_note_popup = false;
                self.new_note_field = NewNoteField::Name;
                if let Err(err) = self.create_new_note(dir.trim(), name.trim(), &tags) {
                    self.status = format!("new note error: {err}");
                }
            }
            KeyCode::Tab => {
                self.new_note_field = match self.new_note_field {
                    NewNoteField::Dir => NewNoteField::Name,
                    NewNoteField::Name => NewNoteField::Tags,
                    NewNoteField::Tags => NewNoteField::Dir,
                };
            }
            KeyCode::Backspace => {
                match self.new_note_field {
                    NewNoteField::Dir => {
                        self.new_note_dir_input.pop();
                    }
                    NewNoteField::Name => {
                        self.new_note_input.pop();
                    }
                    NewNoteField::Tags => {
                        self.new_note_tags_input.pop();
                    }
                }
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                match self.new_note_field {
                    NewNoteField::Dir => self.new_note_dir_input.push(c),
                    NewNoteField::Name => self.new_note_input.push(c),
                    NewNoteField::Tags => self.new_note_tags_input.push(c),
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn suggest_new_note_dir(&self) -> String {
        match self.focus {
            FocusPane::Notes => {
                if let Some(NotesTreeRow::Dir { path, .. }) = self.notes_tree_rows.get(self.selected_note) {
                    path.clone()
                } else if let Some(note) = self.selected_tree_note() {
                    note.slug.rsplit_once('/').map(|(dir, _)| dir.to_string()).unwrap_or_default()
                } else {
                    String::new()
                }
            }
            FocusPane::Preview => self
                .current_note_slug
                .as_deref()
                .and_then(|slug| slug.rsplit_once('/').map(|(dir, _)| dir.to_string()))
                .unwrap_or_default(),
            FocusPane::Links => String::new(),
        }
    }

    fn handle_help_popup_input(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
            self.help_popup = false;
        }
        Ok(())
    }

    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        if command.is_empty() {
            return Ok(());
        }

        let mut parts = command.split_whitespace();
        let Some(cmd) = parts.next() else {
            return Ok(());
        };

        match cmd {
            "w" => self.save_current_note()?,
            "q" => {
                if self.dirty {
                    self.status = "unsaved changes; use :q! to discard or :w".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            "q!" => {
                self.should_quit = true;
            }
            "wq" => {
                self.save_current_note()?;
                self.should_quit = true;
            }
            "e" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.is_empty() {
                    self.status = "usage: :e <slug>".to_string();
                } else {
                    self.open_note_by_slug(&rest)?;
                }
            }
            "e!" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.is_empty() {
                    self.status = "usage: :e! <slug>".to_string();
                } else {
                    self.open_note_by_slug_force(&rest, true)?;
                }
            }
            "r" => {
                self.reload_notes()?;
            }
            "del" => {
                let title = parts.collect::<Vec<_>>().join(" ");
                if title.trim().is_empty() {
                    self.status = "usage: :del <title>".to_string();
                } else {
                    self.delete_note_by_title(&title)?;
                }
            }
            "tags" => {
                let sub = parts.next().unwrap_or_default();
                match sub {
                    "add" => {
                        let rest = parts.collect::<Vec<_>>().join(" ");
                        if rest.trim().is_empty() {
                            self.status = "usage: :tags add <tags...>".to_string();
                        } else {
                            self.add_tags_to_current_note(&rest)?;
                        }
                    }
                    "list" => {
                        self.list_tags_for_current_note()?;
                    }
                    _ => {
                        self.status = "usage: :tags <add|list> ...".to_string();
                    }
                }
            }
            _ => {
                self.status = format!("unknown command: {cmd}");
            }
        }

        Ok(())
    }

    fn handle_preview_normal_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.pending_normal_op = None;
                self.status = "-- NORMAL --".to_string();
                return Ok(true);
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                self.pending_normal_op = None;
                return Ok(true);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.move_cursor_right();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor_up();
            }
            KeyCode::Char('b') => {
                self.move_word_backward();
            }
            KeyCode::Char('e') => {
                self.move_word_end_forward();
            }
            KeyCode::Char('0') => {
                self.cursor_col = 0;
            }
            KeyCode::Char('$') => {
                self.cursor_col = self.current_line_len_chars();
            }
            KeyCode::Char('i') => {
                self.enter_insert_mode();
            }
            KeyCode::Char('v') => {
                self.enter_visual_mode();
            }
            KeyCode::Char('u') => {
                self.undo();
            }
            KeyCode::Char('p') => {
                self.paste_after_cursor();
            }
            KeyCode::Char('z') => {
                self.toggle_current_heading_fold();
                return Ok(true);
            }
            KeyCode::Char('a') => {
                if self.cursor_col < self.current_line_len_chars() {
                    self.cursor_col += 1;
                }
                self.enter_insert_mode();
            }
            KeyCode::Char('A') => {
                self.cursor_col = self.current_line_len_chars();
                self.enter_insert_mode();
            }
            KeyCode::Char('I') => {
                self.cursor_col = self
                    .current_line_ref()
                    .map(|line| first_non_space_char_idx(line.as_str()))
                    .unwrap_or(0);
                self.enter_insert_mode();
            }
            KeyCode::Char('o') => {
                self.ensure_has_line();
                self.push_undo_snapshot();
                let insert_at = self.cursor_row + 1;
                self.editor_lines.insert(insert_at, String::new());
                self.cursor_row = insert_at;
                self.cursor_col = 0;
                self.dirty = true;
                self.enter_insert_mode();
                self.on_editor_content_changed();
            }
            KeyCode::Char('O') => {
                self.ensure_has_line();
                self.push_undo_snapshot();
                let insert_at = self.cursor_row;
                self.editor_lines.insert(insert_at, String::new());
                self.cursor_col = 0;
                self.dirty = true;
                self.enter_insert_mode();
                self.on_editor_content_changed();
            }
            KeyCode::Char('x') => {
                self.delete_char_under_cursor();
            }
            KeyCode::Char('d') => {
                if self.pending_normal_op == Some('d') {
                    self.delete_current_line();
                    self.pending_normal_op = None;
                } else {
                    self.pending_normal_op = Some('d');
                    self.status = "d pending (use dd to delete line)".to_string();
                }
                self.clamp_cursor();
                return Ok(true);
            }
            KeyCode::Char('y') => {
                if self.pending_normal_op == Some('y') {
                    self.yank_current_line();
                    self.pending_normal_op = None;
                } else {
                    self.pending_normal_op = Some('y');
                    self.status = "y pending (use yy to yank line)".to_string();
                }
                self.clamp_cursor();
                return Ok(true);
            }
            _ => {
                self.pending_normal_op = None;
                return Ok(false);
            }
        }

        self.pending_normal_op = None;
        self.clamp_cursor();
        Ok(true)
    }

    fn handle_visual_mode_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') => {
                self.exit_visual_mode();
                return Ok(());
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                return Ok(());
            }
            KeyCode::Char('h') | KeyCode::Left => self.move_cursor_left(),
            KeyCode::Char('l') | KeyCode::Right => self.move_cursor_right(),
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor_up(),
            KeyCode::Char('b') => self.move_word_backward(),
            KeyCode::Char('e') => self.move_word_end_forward(),
            KeyCode::Char('0') => self.cursor_col = 0,
            KeyCode::Char('$') => self.cursor_col = self.current_line_len_chars(),
            KeyCode::Char('d') | KeyCode::Char('x') => {
                self.delete_visual_selection();
                return Ok(());
            }
            KeyCode::Char('y') => {
                self.yank_visual_selection();
                return Ok(());
            }
            KeyCode::Char('u') => {
                self.undo();
                return Ok(());
            }
            _ => {}
        }

        self.clamp_cursor();
        Ok(())
    }

    fn handle_insert_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.editor_mode = EditorMode::Normal;
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
                self.status = "-- NORMAL --".to_string();
            }
            KeyCode::Enter => {
                self.split_line_at_cursor();
            }
            KeyCode::Backspace => {
                self.backspace_in_insert_mode();
            }
            KeyCode::Delete => {
                self.delete_in_insert_mode();
            }
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Tab => {
                self.insert_char('\t');
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert_char(c);
            }
            _ => {}
        }

        self.clamp_cursor();
    }

    fn move_down(&mut self) {
        match self.focus {
            FocusPane::Notes => {
                if self.selected_note + 1 < self.notes_tree_rows.len() {
                    self.selected_note += 1;
                }
            }
            FocusPane::Links => {
                if self.selected_link + 1 < self.links.len() {
                    self.selected_link += 1;
                }
            }
            FocusPane::Preview => self.move_cursor_down(),
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            FocusPane::Notes => {
                self.selected_note = self.selected_note.saturating_sub(1);
            }
            FocusPane::Links => {
                self.selected_link = self.selected_link.saturating_sub(1);
            }
            FocusPane::Preview => self.move_cursor_up(),
        }
    }

}
