use std::path::PathBuf;

pub struct AppDir {
    root: PathBuf,
}

impl AppDir {
    pub fn new() -> anyhow::Result<Self> {
        let root = dirs::config_dir()
            .or_else(dirs::home_dir)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No config/home
directory"
                )
            })?
            .join("monux");

        Ok(Self { root })
    }

    pub fn init(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn index_path(&self) -> PathBuf {
        self.root.join("notes.redb")
    }
}
