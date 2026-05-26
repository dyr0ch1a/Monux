use super::{app_dir::AppDir, config::Config};
use super::watch::VaultWatcher;
use super::storage::NoteStorage;
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
        let config = Config::load(self.config_path())?;
        let index = NoteIndex::open(self.index_path(),
config.notes_dir)?;
        let storage = self.open_note_storage()?;
        for rel in storage.list_note_paths()? {
            index.reindex_note(rel)?;
        }
        let _ = index.prune_orphan_tags()?;
        Ok(index)
    }


    pub fn load_config(&self) -> anyhow::Result<Config> {
        Config::load(self.config_path())
    }


    pub fn open_vault_watcher(&self) -> anyhow::Result<VaultWatcher> {
        let config = Config::load(self.config_path())?;
        VaultWatcher::start(&config.notes_dir)
    }


    pub fn open_note_storage(&self) -> anyhow::Result<NoteStorage> {
        let config = Config::load(self.config_path())?;
        Ok(NoteStorage::new(config.notes_dir))
    }
}


