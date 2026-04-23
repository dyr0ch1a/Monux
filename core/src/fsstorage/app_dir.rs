use std::path::PathBuf;

pub struct AppDir {
    root: PathBuf,
}

impl AppDir {
    pub fn new() -> anyhow::Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;

        Ok(Self {
            root: home.join(".monux"),
        })
    }

    pub fn init(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }
}
