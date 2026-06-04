impl App {
    pub fn on_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.new_note_popup {
            return self.handle_new_note_popup_input(key);
        }
        if self.global_search_popup {
            return self.handle_global_search_input(key);
        }
        if self.new_dir_popup {
            return self.handle_new_dir_popup_input(key);
        }
        if self.rename_popup {
            return self.handle_rename_popup_input(key);
        }
        if self.delete_dir_confirm_popup {
            return self.handle_delete_dir_confirm_input(key);
        }
        if self.help_popup {
            return self.handle_help_popup_input(key);
        }
        if self.command_mode {
            return self.handle_command_input(key);
        }
        if self.search_mode {
            return self.handle_search_input(key);
        }


        if self.editor_mode == EditorMode::Normal &&
self.handle_leader_key(key)? {
            return Ok(());
        }


        if key.modifiers.contains(KeyModifiers::CONTROL) &&
matches!(key.code, KeyCode::Char('s')) {
            self.save_current_note()?;
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) &&
matches!(key.code, KeyCode::Char('r')) {
            self.open_rename_popup();
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) &&
matches!(key.code, KeyCode::Char('n')) {
            self.new_note_popup = true;
            self.new_note_dir_input = self.suggest_new_note_dir();
            self.new_note_input.clear();
            self.new_note_tags_input.clear();
            self.new_note_field = NewNoteField::Name;
            self.status = "new note".to_string();
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) &&
matches!(key.code, KeyCode::Char('f')) {
            self.global_search_popup = true;
            self.global_search_input.clear();
            self.global_search_selected = 0;
            self.global_search_results.clear();
            self.refresh_global_search_results()?;
            self.status = "global search".to_string();
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('D') |
KeyCode::Char('d'))
        {
            self.new_dir_popup = true;
            self.new_dir_input = self.suggest_new_dir_parent();
            self.status = "new directory".to_string();
            return Ok(());
        }
        if self.focus == FocusPane::Preview && self.editor_mode ==
EditorMode::Insert {
            self.handle_insert_mode_key(key);
            return Ok(());
        }
        if self.focus == FocusPane::Preview
            && matches!(key.code, KeyCode::Char('p'))
            && !self.notes_tree_cut_paths.is_empty()
        {
            self.paste_cut_notes_to_selected_dir()?;
            return Ok(());
        }


        if self.editor_mode == EditorMode::Normal
            && !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Char('?'))
        {
            self.help_popup = true;
            self.help_popup_scroll = 0;
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
            KeyCode::Esc => {
                if self.focus == FocusPane::Notes &&
self.notes_tree_visual_mode {
                    self.exit_notes_tree_visual_mode();
                    return Ok(());
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
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_input.clear();
                self.status = if self.focus == FocusPane::Preview {
                    "search in note".to_string()
                } else {
                    "search notes tree".to_string()
                };
                return Ok(());
            }
            KeyCode::Enter => match self.focus {
                FocusPane::Notes =>
self.activate_selected_note_tree_row()?,
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
                    if
matches!(self.notes_tree_rows.get(self.selected_note),
Some(NotesTreeRow::Dir { .. })) {
                        self.prompt_delete_selected_dir();
                    } else {
                        self.delete_selected_note()?;
                    }
                }
            }
            KeyCode::Char('v') => {
                if self.focus == FocusPane::Notes {
                    if self.notes_tree_visual_mode {
                        self.exit_notes_tree_visual_mode();
                    } else {
                        self.enter_notes_tree_visual_mode();
                    }
                }
            }
            KeyCode::Char('x') => {
                if self.focus == FocusPane::Notes {
                    self.cut_notes_tree_selection();
                    return Ok(());
                }
            }
            KeyCode::Char('p') => {
                if self.focus == FocusPane::Notes {
                    self.paste_cut_notes_to_selected_dir()?;
                    return Ok(());
                }
            }
            KeyCode::Char('r') => {
                self.reload_notes()?;
            }
            _ => {}
        }


        Ok(())
    }


    fn handle_leader_key(&mut self, key: KeyEvent) ->
anyhow::Result<bool> {
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
            self.status = "leader: <space>e notes, <space>b
links".to_string();
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
            || (self.focus == FocusPane::Links &&
!self.show_links_panel)
        {
            self.focus = FocusPane::Preview;
        }
    }


    fn handle_command_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
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


    fn handle_search_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_input.clear();
                self.status = "search cancelled".to_string();
            }
            KeyCode::Enter => {
                if self.focus == FocusPane::Preview {
                    if self.apply_in_note_search() {
                        self.last_note_search = self.search_input.trim().to_string();
                        self.status = format!("found: {}", self.search_input.trim());
                    } else {
                        self.status = format!("not found: {}", self.search_input.trim());
                    }
                    self.search_mode = false;
                } else {
                    self.search_mode = false;
                    self.status = format!("search: {}", self.search_input.trim());
                }
            }
            KeyCode::Backspace => {
                self.search_input.pop();
                if self.focus == FocusPane::Preview {
                    let _ = self.apply_in_note_search();
                } else {
                    self.apply_notes_tree_search();
                }
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.search_input.push(c);
                if self.focus == FocusPane::Preview {
                    let _ = self.apply_in_note_search();
                } else {
                    self.apply_notes_tree_search();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_in_note_search(&mut self) -> bool {
        let q = self.search_input.trim().to_lowercase();
        self.find_in_note(&q, true, false)
    }

    fn find_in_note(&mut self, q: &str, forward: bool, exclusive: bool) -> bool {
        if q.is_empty() || self.editor_lines.is_empty() {
            return false;
        }

        let start_row = self.cursor_row.min(self.editor_lines.len().saturating_sub(1));
        let start_col = self.cursor_col;
        let start_byte = self
            .current_line_ref()
            .map(|line| {
                let col = if exclusive { start_col.saturating_add(1) } else { start_col };
                char_to_byte_idx(line, col.min(line.chars().count()))
            })
            .unwrap_or(0);

        if forward {
            for row in start_row..self.editor_lines.len() {
                let line = &self.editor_lines[row];
                let line_l = line.to_lowercase();
                let from = if row == start_row { start_byte.min(line_l.len()) } else { 0 };
                if let Some(found) = line_l[from..].find(q) {
                    let byte = from + found;
                    self.cursor_row = row;
                    self.cursor_col = line[..byte].chars().count();
                    self.clamp_cursor();
                    return true;
                }
            }
            for row in 0..start_row {
                let line = &self.editor_lines[row];
                let line_l = line.to_lowercase();
                if let Some(found) = line_l.find(q) {
                    self.cursor_row = row;
                    self.cursor_col = line[..found].chars().count();
                    self.clamp_cursor();
                    return true;
                }
            }
        } else {
            for row in (0..=start_row).rev() {
                let line = &self.editor_lines[row];
                let line_l = line.to_lowercase();
                let until = if row == start_row {
                    start_byte.min(line_l.len())
                } else {
                    line_l.len()
                };
                let slice = &line_l[..until];
                if let Some((found, _)) = slice.rmatch_indices(q).next() {
                    self.cursor_row = row;
                    self.cursor_col = line[..found].chars().count();
                    self.clamp_cursor();
                    return true;
                }
            }
            for row in ((start_row + 1)..self.editor_lines.len()).rev() {
                let line = &self.editor_lines[row];
                let line_l = line.to_lowercase();
                if let Some((found, _)) = line_l.rmatch_indices(q).next() {
                    self.cursor_row = row;
                    self.cursor_col = line[..found].chars().count();
                    self.clamp_cursor();
                    return true;
                }
            }
        }

        false
    }


    fn handle_global_search_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.global_search_popup = false;
                self.global_search_input.clear();
                self.global_search_results.clear();
                self.global_search_selected = 0;
                self.status = "global search cancelled".to_string();
            }
            KeyCode::Enter => {
                self.apply_global_search_selection()?;
            }
            KeyCode::Backspace => {
                self.global_search_input.pop();
                self.refresh_global_search_results()?;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.global_search_selected + 1 <
self.global_search_results.len() {
                    self.global_search_selected += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.global_search_selected =
self.global_search_selected.saturating_sub(1);
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.global_search_input.push(c);
                self.refresh_global_search_results()?;
            }
            _ => {}
        }
        Ok(())
    }


    fn refresh_global_search_results(&mut self) -> anyhow::Result<()>
{
        let q = self.global_search_input.trim().to_lowercase();
        let mut results = Vec::new();
        let mut seen_notes = std::collections::HashSet::new();


        if q.is_empty() {
            for note in self.notes.iter().take(20) {
                if seen_notes.insert(note.path.clone()) {
                    results.push(GlobalSearchResult::Note {
                        path: note.path.clone(),
                        matched_by_tag: false,
                    });
                }
            }
        } else {
            let parts = q
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            let name_query = parts.first().copied().unwrap_or("");
            let required_tags = if parts.len() > 1 {
                parts[1..].to_vec()
            } else {
                Vec::new()
            };

            for dir in self.indexed_dirs() {
                if required_tags.is_empty() && !name_query.is_empty() && dir.to_lowercase().contains(name_query) {
                    results.push(GlobalSearchResult::Dir(dir));
                }
            }


            for note in &self.notes {
                let path_l = note.display_path().to_lowercase();
                let title_l = note.title.to_lowercase();
                let tags = self.storage.read_tags(&note.path)?;
                let tags_l = tags
                    .iter()
                    .map(|tag| tag.to_lowercase())
                    .collect::<Vec<_>>();
                let name_match = name_query.is_empty()
                    || path_l.contains(name_query)
                    || title_l.contains(name_query);
                let tags_match = required_tags
                    .iter()
                    .all(|required| tags_l.iter().any(|tag| tag == required));

                if name_match && tags_match && seen_notes.insert(note.path.clone()) {
                    let matched_by_tag = !required_tags.is_empty();
                    results.push(GlobalSearchResult::Note {
                        path: note.path.clone(),
                        matched_by_tag,
                    });
                }
            }
        }


        self.global_search_results =
results.into_iter().take(80).collect();
        if self.global_search_selected >=
self.global_search_results.len() {
            self.global_search_selected =
self.global_search_results.len().saturating_sub(1);
        }
        Ok(())
    }


    fn apply_global_search_selection(&mut self) -> anyhow::Result<()>
{
        let Some(item) =
self.global_search_results.get(self.global_search_selected).cloned() else {
            return Ok(());
        };


        self.global_search_popup = false;
        self.global_search_input.clear();


        match item {
            GlobalSearchResult::Dir(path) => {
                self.show_notes_panel = true;
                self.focus = FocusPane::Notes;
                self.enter_notes_dir(&path);
                self.status = format!("entered {path}/");
            }
            GlobalSearchResult::Note { path, .. } => {
                self.show_notes_panel = true;
                self.focus = FocusPane::Notes;
                self.open_note_by_path(&path)?;
            }
        }


        self.global_search_results.clear();
        self.global_search_selected = 0;
        Ok(())
    }


    fn handle_new_note_popup_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
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
                let dir = std::mem::take(&mut
self.new_note_dir_input);
                let name = std::mem::take(&mut self.new_note_input);
                let tags = std::mem::take(&mut
self.new_note_tags_input);
                self.new_note_popup = false;
                self.new_note_field = NewNoteField::Name;
                if let Err(err) = self.create_new_note(dir.trim(),
name.trim(), &tags) {
                    self.status = format!("new note error: {err}");
                }
            }
            KeyCode::BackTab => {
                self.new_note_field = match self.new_note_field {
                    NewNoteField::Dir => NewNoteField::Tags,
                    NewNoteField::Name => NewNoteField::Dir,
                    NewNoteField::Tags => NewNoteField::Name,
                };
            }
            KeyCode::Tab => {
                let shift_pressed =
key.modifiers.contains(KeyModifiers::SHIFT);
                self.new_note_field = if shift_pressed {
                    match self.new_note_field {
                        NewNoteField::Dir => NewNoteField::Tags,
                        NewNoteField::Name => NewNoteField::Dir,
                        NewNoteField::Tags => NewNoteField::Name,
                    }
                } else {
                    match self.new_note_field {
                        NewNoteField::Dir => NewNoteField::Name,
                        NewNoteField::Name => NewNoteField::Tags,
                        NewNoteField::Tags => NewNoteField::Dir,
                    }
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
                    NewNoteField::Dir =>
self.new_note_dir_input.push(c),
                    NewNoteField::Name => self.new_note_input.push(c),
                    NewNoteField::Tags =>
self.new_note_tags_input.push(c),
                }
            }
            _ => {}
        }
        Ok(())
    }


    fn suggest_new_note_dir(&self) -> String {
        match self.focus {
            FocusPane::Notes => {
                if let Some(NotesTreeRow::Dir { path, .. }) =
self.notes_tree_rows.get(self.selected_note) {
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
                .and_then(|rel| rel.parent().and_then(|p|
p.to_str()).map(str::to_string))
                .unwrap_or_default(),
            FocusPane::Links => String::new(),
        }
    }


    fn handle_help_popup_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                self.help_popup = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.help_popup_scroll =
self.help_popup_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.help_popup_scroll =
self.help_popup_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.help_popup_scroll =
self.help_popup_scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                self.help_popup_scroll =
self.help_popup_scroll.saturating_add(8);
            }
            KeyCode::Home => {
                self.help_popup_scroll = 0;
            }
            _ => {}
        }
        Ok(())
    }




    fn execute_command(&mut self, command: &str) -> anyhow::Result<()>
{
        if command.is_empty() {
            return Ok(());
        }


        let mut parts = command.split_whitespace();
        let Some(cmd) = parts.next() else {
            return Ok(());
        };


        match cmd {
            "w" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.trim().is_empty() {
                    self.save_current_note()?;
                } else {
                    self.save_current_note_as(&rest)?;
                }
            }
            "q" => {
                if self.dirty {
                    self.status = "unsaved changes; use :q! to discard
or :w".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            "q!" => {
                self.should_quit = true;
            }
            "wq" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.trim().is_empty() {
                    if self.current_note_rel.is_none() {
                        self.status = "no note name; use :w <name> or
:wq <name>".to_string();
                    } else {
                        self.save_current_note()?;
                        self.should_quit = true;
                    }
                } else {
                    self.save_current_note_as(&rest)?;
                    if self.current_note_rel.is_some() {
                        self.should_quit = true;
                    }
                }
            }
            "e" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.is_empty() {
                    self.status = "usage: :e <path>".to_string();
                } else {
                    let rel = resolve_note_path(&rest);
                    self.open_note_by_path(&rel)?;
                }
            }
            "e!" => {
                let rest = parts.collect::<Vec<_>>().join(" ");
                if rest.is_empty() {
                    self.status = "usage: :e! <path>".to_string();
                } else {
                    let rel = resolve_note_path(&rest);
                    self.open_note_by_path_force(&rel, true)?;
                }
            }
            "r" => {
                self.reload_notes()?;
            }
            "sync" => {
                let dir = parts.collect::<Vec<_>>().join(" ");
                let filter = if dir.trim().is_empty() {
                    None
                } else {
                    Some(dir.trim())
                };
                self.sync_notes_index(filter)?;
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
                self.status = "tags popup removed".to_string();
            }
            "mkdir" => {
                let path = parts.collect::<Vec<_>>().join(" ");
                if path.trim().is_empty() {
                    self.status = "usage: :mkdir
<dir/path>".to_string();
                } else if let Err(err) = self.create_directory(&path)
{
                    self.status = format!("mkdir error: {err}");
                }
            }
            "mv" => {
                let dir = parts.collect::<Vec<_>>().join(" ");
                if dir.trim().is_empty() {
                    self.status = "usage: :mv <dir/path>".to_string();
                } else if let Err(err) =
self.move_selected_or_current_note_to_dir(&dir) {
                    self.status = format!("mv error: {err}");
                }
            }
            _ => {
                self.status = format!("unknown command: {cmd}");
            }
        }


        Ok(())
    }


    fn handle_preview_normal_key(&mut self, key: KeyEvent) ->
anyhow::Result<bool> {
        if self.pending_normal_op == Some('d') {
            match key.code {
                KeyCode::Char('b') | KeyCode::Char('w') |
KeyCode::Char('e') => {
                    let start = (self.cursor_row, self.cursor_col);
                    let count = self.take_motion_count();
                    match key.code {
                        KeyCode::Char('b') => {
                            self.repeat_motion(count, |app|
app.move_word_backward());
                            let end = (self.cursor_row,
self.cursor_col);
                            self.delete_motion_range(start, end,
false);
                        }
                        KeyCode::Char('w') => {
                            self.repeat_motion(count, |app|
app.move_word_forward());
                            let end = (self.cursor_row,
self.cursor_col);
                            self.delete_motion_range(start, end,
false);
                        }
                        KeyCode::Char('e') => {
                            self.repeat_motion(count, |app|
app.move_word_end_forward());
                            let end = (self.cursor_row,
self.cursor_col);
                            self.delete_motion_range(start, end,
true);
                        }
                        _ => {}
                    }
                    self.pending_normal_op = None;
                    self.clamp_cursor();
                    return Ok(true);
                }
                _ => {}
            }
        }


        if let KeyCode::Char(ch) = key.code {
            if ch.is_ascii_digit() {
                let digit = ch as u8 - b'0';
                if self.push_motion_digit(digit) {
                    return Ok(true);
                }
                if digit == 0 && self.motion_count.is_none() {
                    self.cursor_col = 0;
                    self.clamp_cursor();
                    return Ok(true);
                }
            }
        }


        match key.code {
            KeyCode::Esc => {
                self.clear_editor_input_state();
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
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_cursor_left());
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_cursor_right());
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_cursor_down());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app| app.move_cursor_up());
            }
            KeyCode::Char('b') => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_word_backward());
            }
            KeyCode::Char('e') => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_word_end_forward());
            }
            KeyCode::Char('w') => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_word_forward());
            }
            KeyCode::Char('n') => {
                let q = self.last_note_search.trim().to_lowercase();
                if q.is_empty() {
                    self.status = "no active search".to_string();
                    return Ok(true);
                }
                if self.find_in_note(&q, true, true) {
                    self.status = format!("found: {}", self.last_note_search);
                } else {
                    self.status = format!("not found: {}", self.last_note_search);
                }
                return Ok(true);
            }
            KeyCode::Char('N') => {
                let q = self.last_note_search.trim().to_lowercase();
                if q.is_empty() {
                    self.status = "no active search".to_string();
                    return Ok(true);
                }
                if self.find_in_note(&q, false, true) {
                    self.status = format!("found: {}", self.last_note_search);
                } else {
                    self.status = format!("not found: {}", self.last_note_search);
                }
                return Ok(true);
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
            KeyCode::Char('V') => {
                self.enter_visual_line_mode();
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
                let len = self.current_line_len_chars();
                if len > 0 && self.cursor_col < len.saturating_sub(1)
{
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
                    .map(|line|
first_non_space_char_idx(line.as_str()))
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
                    let count = self.take_motion_count();
                    self.repeat_motion(count, |app|
app.delete_current_line());
                    self.pending_normal_op = None;
                } else {
                    self.pending_normal_op = Some('d');
                    self.status = "d pending (dd, db, dw,
de)".to_string();
                }
                self.clamp_cursor();
                return Ok(true);
            }
            KeyCode::Char('y') => {
                if self.pending_normal_op == Some('y') {
                    let count = self.take_motion_count();
                    for _ in 0..count {
                        self.yank_current_line();
                        if self.cursor_row + 1 <
self.editor_lines.len() {
                            self.cursor_row += 1;
                        }
                    }
                    self.pending_normal_op = None;
                } else {
                    self.pending_normal_op = Some('y');
                    self.status = "y pending (use yy to yank
line)".to_string();
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


    fn handle_visual_mode_key(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
        if let KeyCode::Char(ch) = key.code {
            if ch.is_ascii_digit() {
                let digit = ch as u8 - b'0';
                if self.push_motion_digit(digit) {
                    return Ok(());
                }
                if digit == 0 && self.motion_count.is_none() {
                    self.cursor_col = 0;
                    self.clamp_cursor();
                    return Ok(());
                }
            }
        }


        match key.code {
            KeyCode::Esc | KeyCode::Char('v') => {
                self.exit_visual_mode();
                return Ok(());
            }
            KeyCode::Char('V') => {
                self.enter_visual_line_mode();
                return Ok(());
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                return Ok(());
            }
            KeyCode::Char('h') | KeyCode::Left => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_cursor_left());
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_cursor_right());
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_cursor_down());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app| app.move_cursor_up());
            }
            KeyCode::Char('b') => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_word_backward());
            }
            KeyCode::Char('e') => {
                let count = self.take_motion_count();
                self.repeat_motion(count, |app|
app.move_word_end_forward());
            }
            KeyCode::Char('0') => self.cursor_col = 0,
            KeyCode::Char('$') => self.cursor_col =
self.current_line_len_chars(),
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
            // Temporary workaround: ignore Tab in insert mode to
            // avoid cursor/render desync.
            KeyCode::Tab | KeyCode::BackTab => {}
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
                if self.selected_note + 1 < self.notes_tree_rows.len()
{
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
                self.selected_note =
self.selected_note.saturating_sub(1);
            }
            FocusPane::Links => {
                self.selected_link =
self.selected_link.saturating_sub(1);
            }
            FocusPane::Preview => self.move_cursor_up(),
        }
    }


    fn open_rename_popup(&mut self) {
        let target = match
self.notes_tree_rows.get(self.selected_note) {
            Some(NotesTreeRow::Dir { path, .. }) =>
Some(crate::app::RenameTarget::Dir(path.clone())),
            Some(NotesTreeRow::Note { note_index, .. }) => self
                .notes
                .get(*note_index)
                .map(|note|
crate::app::RenameTarget::Note(note.path.clone())),
            _ => None,
        }.or_else(|| {
            self.current_note_rel
                .as_ref()
                .cloned()
                .map(crate::app::RenameTarget::Note)
        });


        let Some(target) = target else {
            self.status = "nothing to rename".to_string();
            return;
        };


        self.rename_target_label = match &target {
            crate::app::RenameTarget::Dir(path) => format!("dir:
{path}/"),
            crate::app::RenameTarget::Note(path) => {
                format!("note: {}", monux_core::index::path_key(path))
            }
        };
        self.rename_input = match &target {
            crate::app::RenameTarget::Dir(path) => path
                .rsplit('/')
                .next()
                .map(str::to_string)
                .unwrap_or_default(),
            crate::app::RenameTarget::Note(path) => path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
                .unwrap_or_default(),
        };
        self.rename_target = Some(target);
        self.rename_popup = true;
        self.status = "rename".to_string();
    }


    fn handle_rename_popup_input(&mut self, key: KeyEvent) ->
anyhow::Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.rename_popup = false;
                self.rename_input.clear();
                self.rename_target_label.clear();
                self.rename_target = None;
                self.status = "rename cancelled".to_string();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.rename_input);
                let target = self.rename_target.take();
                self.rename_popup = false;
                self.rename_target_label.clear();
                match target {
                    Some(crate::app::RenameTarget::Note(path)) => {
                        if let Err(err) =
self.rename_note_by_path(&path, &input) {
                            self.status = format!("rename error:
{err}");
                        }
                    }
                    Some(crate::app::RenameTarget::Dir(old_dir)) => {
                        if let Err(err) =
self.rename_directory_from_path(&old_dir, &input) {
                            self.status = format!("renamedir error:
{err}");
                        }
                    }
                    None => {
                        self.status = "nothing to rename".to_string();
                    }
                }
            }
            KeyCode::Backspace => {
                self.rename_input.pop();
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.rename_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }


}
