use std::path::PathBuf;

use crate::editor::{
    address::{Addr, AddressRange},
    command::{Command, parse_command},
    error::EditorError,
    event::Event,
    storage::Storage,
};

pub struct ExecOutcome {
    pub events: Vec<Event>,
    pub should_quit: bool,
}

pub struct Editor<S: Storage> {
    storage: S,
    lines: Vec<String>,
    current: usize,
    modified: bool,
    file_path: Option<PathBuf>,
    pending: Option<PendingInput>,
}

enum PendingInput {
    Append {
        after: usize,
        lines: Vec<String>,
    },
    Change {
        start: usize,
        end: usize,
        lines: Vec<String>,
    },
}

impl<S: Storage> Editor<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            lines: Vec::new(),
            current: 0,
            modified: false,
            file_path: None,
            pending: None,
        }
    }

    pub fn is_in_input_mode(&self) -> bool {
        self.pending.is_some()
    }

    pub fn execute(&mut self, input: &str) -> Result<ExecOutcome, EditorError> {
        if self.pending.is_some() {
            return self.feed_input_line(input);
        }

        let cmd = parse_command(input)?;
        self.apply_command(cmd)
    }

    fn feed_input_line(&mut self, input: &str) -> Result<ExecOutcome, EditorError> {
        if input == "." {
            let pending = self.pending.take().ok_or_else(|| {
                EditorError::InvalidCommand("expected pending input mode".to_string())
            })?;

            match pending {
                PendingInput::Append { after, lines } => {
                    self.insert_after(after, lines);
                }
                PendingInput::Change { start, end, lines } => {
                    self.replace_range(start, end, lines)?;
                }
            }

            self.modified = true;
            return Ok(ExecOutcome {
                events: vec![Event::Message(format!("{}", self.current))],
                should_quit: false,
            });
        }

        match self.pending.as_mut() {
            Some(PendingInput::Append { lines, .. }) => lines.push(input.to_string()),
            Some(PendingInput::Change { lines, .. }) => lines.push(input.to_string()),
            None => {
                return Err(EditorError::InvalidCommand(
                    "unexpected input state".to_string(),
                ));
            }
        }

        Ok(ExecOutcome {
            events: Vec::new(),
            should_quit: false,
        })
    }

    fn apply_command(&mut self, cmd: Command) -> Result<ExecOutcome, EditorError> {
        match cmd {
            Command::Print { range, numbered } => {
                let (start, end) = self.resolve_range(range, true)?;
                let mut events = Vec::new();

                for idx in start..=end {
                    let line = self.lines[idx - 1].clone();
                    if numbered {
                        events.push(Event::Line(format!("{idx}\t{line}")));
                    } else {
                        events.push(Event::Line(line));
                    }
                }

                self.current = end;
                Ok(ExecOutcome {
                    events,
                    should_quit: false,
                })
            }
            Command::Delete { range } => {
                let (start, end) = self.resolve_range(range, true)?;
                self.delete_range(start, end)?;
                self.modified = true;
                Ok(ExecOutcome {
                    events: vec![Event::Message(format!("{}", self.current))],
                    should_quit: false,
                })
            }
            Command::Append { at } => {
                let at = self.resolve_single_for_insert(at, InsertKind::After)?;
                self.pending = Some(PendingInput::Append {
                    after: at,
                    lines: Vec::new(),
                });
                Ok(ExecOutcome {
                    events: vec![Event::Message(
                        "enter input mode, finish with '.'".to_string(),
                    )],
                    should_quit: false,
                })
            }
            Command::Insert { at } => {
                let target = self.resolve_single_for_insert(at, InsertKind::Before)?;
                let after = target.saturating_sub(1);
                self.pending = Some(PendingInput::Append {
                    after,
                    lines: Vec::new(),
                });
                Ok(ExecOutcome {
                    events: vec![Event::Message(
                        "enter input mode, finish with '.'".to_string(),
                    )],
                    should_quit: false,
                })
            }
            Command::Change { range } => {
                let (start, end) = self.resolve_range(range, true)?;
                self.pending = Some(PendingInput::Change {
                    start,
                    end,
                    lines: Vec::new(),
                });
                Ok(ExecOutcome {
                    events: vec![Event::Message(
                        "enter input mode, finish with '.'".to_string(),
                    )],
                    should_quit: false,
                })
            }
            Command::Write { path } => {
                let path = match path {
                    Some(path) => {
                        self.file_path = Some(path.clone());
                        path
                    }
                    None => self.file_path.clone().ok_or(EditorError::MissingFilename)?,
                };

                let mut data = self.lines.join("\n");
                if !data.is_empty() {
                    data.push('\n');
                }

                self.storage.write_string(&path, &data)?;
                self.modified = false;

                Ok(ExecOutcome {
                    events: vec![Event::Message(format!("{}", data.len()))],
                    should_quit: false,
                })
            }
            Command::Edit { path } => {
                if self.modified {
                    return Err(EditorError::UnsavedChanges);
                }

                let path = match path {
                    Some(path) => {
                        self.file_path = Some(path.clone());
                        path
                    }
                    None => self.file_path.clone().ok_or(EditorError::MissingFilename)?,
                };

                let content = self.storage.read_to_string(&path)?;
                self.lines = split_lines(&content);
                self.current = self.lines.len();
                self.modified = false;

                Ok(ExecOutcome {
                    events: vec![Event::Message(format!("{}", self.lines.len()))],
                    should_quit: false,
                })
            }
            Command::Quit { force } => {
                if self.modified && !force {
                    return Err(EditorError::UnsavedChanges);
                }

                Ok(ExecOutcome {
                    events: Vec::new(),
                    should_quit: true,
                })
            }
        }
    }

    fn resolve_range(
        &self,
        range: Option<AddressRange>,
        require_non_empty: bool,
    ) -> Result<(usize, usize), EditorError> {
        if self.lines.is_empty() {
            if require_non_empty {
                return Err(EditorError::EmptyBuffer);
            }
            return Ok((0, 0));
        }

        let (start_addr, end_addr) = if let Some(range) = range {
            (range.start, range.end)
        } else {
            (Addr::Current, Addr::Current)
        };

        let start = self.resolve_addr(start_addr)?;
        let end = self.resolve_addr(end_addr)?;

        if start == 0 || end == 0 || start > self.lines.len() || end > self.lines.len() {
            return Err(EditorError::OutOfRange);
        }

        if start > end {
            return Err(EditorError::InvalidAddress("start > end".to_string()));
        }

        Ok((start, end))
    }

    fn resolve_single_for_insert(
        &self,
        range: Option<AddressRange>,
        kind: InsertKind,
    ) -> Result<usize, EditorError> {
        if self.lines.is_empty() {
            return Ok(0);
        }

        let addr = match range {
            Some(range) => {
                if self.resolve_addr(range.start)? != self.resolve_addr(range.end)? {
                    return Err(EditorError::InvalidAddress(
                        "insert/append takes single address".to_string(),
                    ));
                }
                range.end
            }
            None => Addr::Current,
        };

        let resolved = self.resolve_addr(addr)?;
        if resolved == 0 || resolved > self.lines.len() {
            return Err(EditorError::OutOfRange);
        }

        match kind {
            InsertKind::After => Ok(resolved),
            InsertKind::Before => Ok(resolved),
        }
    }

    fn resolve_addr(&self, addr: Addr) -> Result<usize, EditorError> {
        match addr {
            Addr::Current => {
                if self.current == 0 {
                    if self.lines.is_empty() {
                        Err(EditorError::EmptyBuffer)
                    } else {
                        Ok(1)
                    }
                } else {
                    Ok(self.current)
                }
            }
            Addr::Last => {
                if self.lines.is_empty() {
                    Err(EditorError::EmptyBuffer)
                } else {
                    Ok(self.lines.len())
                }
            }
            Addr::Absolute(n) => Ok(n),
        }
    }

    fn insert_after(&mut self, after: usize, mut new_lines: Vec<String>) {
        let insert_at = after.min(self.lines.len());
        let count = new_lines.len();
        self.lines.splice(insert_at..insert_at, new_lines.drain(..));
        self.current = if count == 0 { after } else { insert_at + count };
    }

    fn replace_range(
        &mut self,
        start: usize,
        end: usize,
        mut new_lines: Vec<String>,
    ) -> Result<(), EditorError> {
        if start == 0 || end == 0 || start > end || end > self.lines.len() {
            return Err(EditorError::OutOfRange);
        }

        let replacement_len = new_lines.len();
        self.lines.splice((start - 1)..end, new_lines.drain(..));

        if replacement_len == 0 {
            if start <= self.lines.len() {
                self.current = start;
            } else {
                self.current = self.lines.len();
            }
        } else {
            self.current = start + replacement_len - 1;
        }

        Ok(())
    }

    fn delete_range(&mut self, start: usize, end: usize) -> Result<(), EditorError> {
        if start == 0 || end == 0 || start > end || end > self.lines.len() {
            return Err(EditorError::OutOfRange);
        }

        self.lines.drain((start - 1)..end);

        if self.lines.is_empty() {
            self.current = 0;
        } else if start <= self.lines.len() {
            self.current = start;
        } else {
            self.current = self.lines.len();
        }

        Ok(())
    }
}

enum InsertKind {
    After,
    Before,
}

fn split_lines(content: &str) -> Vec<String> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<String> = content.split('\n').map(|line| line.to_string()).collect();
    if content.ends_with('\n') {
        lines.pop();
    }

    lines
}
