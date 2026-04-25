use std::path::PathBuf;

use crate::editor::{
    address::{AddressRange, parse_address_range},
    error::EditorError,
};

#[derive(Debug)]
pub enum Command {
    Print {
        range: Option<AddressRange>,
        numbered: bool,
    },
    Delete {
        range: Option<AddressRange>,
    },
    Append {
        at: Option<AddressRange>,
    },
    Insert {
        at: Option<AddressRange>,
    },
    Change {
        range: Option<AddressRange>,
    },
    Write {
        path: Option<PathBuf>,
    },
    Edit {
        path: Option<PathBuf>,
    },
    Quit {
        force: bool,
    },
}

pub fn parse_command(input: &str) -> Result<Command, EditorError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Command::Print {
            range: None,
            numbered: false,
        });
    }

    let (range, consumed) = parse_address_range(trimmed)?
        .map(|(r, n)| (Some(r), n))
        .unwrap_or((None, 0));

    let body = trimmed[consumed..].trim_start();
    if body.is_empty() {
        return Ok(Command::Print {
            range,
            numbered: false,
        });
    }

    let mut chars = body.chars();
    let cmd = chars
        .next()
        .ok_or_else(|| EditorError::InvalidCommand(body.to_string()))?;
    let arg = chars.as_str().trim();

    match cmd {
        'p' => Ok(Command::Print {
            range,
            numbered: false,
        }),
        'n' => Ok(Command::Print {
            range,
            numbered: true,
        }),
        'd' => Ok(Command::Delete { range }),
        'a' => Ok(Command::Append { at: range }),
        'i' => Ok(Command::Insert { at: range }),
        'c' => Ok(Command::Change { range }),
        'w' => Ok(Command::Write {
            path: parse_path(arg),
        }),
        'e' => Ok(Command::Edit {
            path: parse_path(arg),
        }),
        'q' => Ok(Command::Quit { force: false }),
        'Q' => Ok(Command::Quit { force: true }),
        _ => Err(EditorError::InvalidCommand(body.to_string())),
    }
}

fn parse_path(arg: &str) -> Option<PathBuf> {
    if arg.is_empty() {
        None
    } else {
        Some(PathBuf::from(arg))
    }
}
