use core::context::StorageContext;
use core::fsstorage::config::Config;

pub struct CommandContext {
    storage: StorageContext,
}

impl CommandContext {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            storage: StorageContext::new()?,
        })
    }

    pub fn init(&self) -> anyhow::Result<()> {
        self.storage.init_app_dir()
    }

    pub fn config_path(&self) -> std::path::PathBuf {
        self.storage.config_path()
    }

    pub fn load_config(&self) -> anyhow::Result<Config> {
        self.storage.load_config()
    }
}
