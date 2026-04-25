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
        'p' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Print {
                range,
                numbered: false,
            })
        }
        'n' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Print {
                range,
                numbered: true,
            })
        }
        'd' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Delete { range })
        }
        'a' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Append { at: range })
        }
        'i' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Insert { at: range })
        }
        'c' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Change { range })
        }
        'w' => Ok(Command::Write {
            path: parse_path(arg),
        }),
        'e' => Ok(Command::Edit {
            path: parse_path(arg),
        }),
        'q' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Quit { force: false })
        }
        'Q' => {
            ensure_no_arg(cmd, arg)?;
            Ok(Command::Quit { force: true })
        }
        _ => Err(EditorError::InvalidCommand(body.to_string())),
    }
}

fn ensure_no_arg(cmd: char, arg: &str) -> Result<(), EditorError> {
    if arg.is_empty() {
        Ok(())
    } else {
        Err(EditorError::InvalidCommand(format!(
            "command '{cmd}' does not take arguments: {arg}"
        )))
    }
}

fn parse_path(arg: &str) -> Option<PathBuf> {
    if arg.is_empty() {
        None
    } else {
        Some(PathBuf::from(arg))
    }
}
