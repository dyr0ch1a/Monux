use std::path::Path;

pub trait Storage {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String>;
    fn write_string(&self, path: &Path, data: &str) -> std::io::Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FsStorage;

impl Storage for FsStorage {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write_string(&self, path: &Path, data: &str) -> std::io::Result<()> {
        std::fs::write(path, data)
    }
}
