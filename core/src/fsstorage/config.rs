use std::path::PathBuf;

pub struct Config {
    pub notes_dir: PathBuf,
    pub plugins_dir: PathBuf,
}

impl Config {
    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        let mut notes_dir = None;
        let mut plugins_dir = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            if key.trim() == "notes_dir" {
                let value = value.trim().trim_matches('"');
                notes_dir = Some(Self::resolve_home(value));
            } else if key.trim() == "plugins_dir" {
                let value = value.trim().trim_matches('"');
                plugins_dir = Some(Self::resolve_home(value));
            }
        }

        let plugins_dir = plugins_dir.unwrap_or_else(|| Self::resolve_home("~/.monux/plugins"));

        Ok(Self {
            notes_dir: notes_dir.ok_or_else(|| anyhow::anyhow!("missing notes_dir"))?,
            plugins_dir,
        })
    }

    fn resolve_home(path: &str) -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                return home;
            }
            if let Some(p) = path.strip_prefix("~/") {
                return home.join(p);
            }
        }
        PathBuf::from(path)
    }
}
