#[derive(Debug)]
pub enum EditorError {
    InvalidAddress(String),
    InvalidCommand(String),
    MissingFilename,
    EmptyBuffer,
    OutOfRange,
    UnsavedChanges,
    Io(std::io::Error),
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAddress(msg) => write!(f, "invalid address: {msg}"),
            Self::InvalidCommand(msg) => write!(f, "invalid command: {msg}"),
            Self::MissingFilename => write!(f, "missing filename"),
            Self::EmptyBuffer => write!(f, "buffer is empty"),
            Self::OutOfRange => write!(f, "address out of range"),
            Self::UnsavedChanges => write!(f, "buffer modified; write first"),
            Self::Io(err) => write!(f, "io error: {err}"),
        }
    }
}

impl std::error::Error for EditorError {}

impl From<std::io::Error> for EditorError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
