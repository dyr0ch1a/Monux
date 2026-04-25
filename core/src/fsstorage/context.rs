use super::{app_dir::AppDir, config::Config};
use crate::index::NoteIndex;

pub struct StorageContext {
    app_dir: AppDir,
}

impl StorageContext {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            app_dir: AppDir::new()?,
        })
    }

    pub fn init_app_dir(&self) -> anyhow::Result<()> {
        self.app_dir.init()
    }

    pub fn config_path(&self) -> std::path::PathBuf {
        self.app_dir.config_path()
    }

    pub fn index_path(&self) -> std::path::PathBuf {
        self.app_dir.index_path()
    }

    pub fn open_note_index(&self) -> anyhow::Result<NoteIndex> {
        NoteIndex::open(self.index_path())
    }

    pub fn load_config(&self) -> anyhow::Result<Config> {
        Config::load(self.config_path())
    }
}
