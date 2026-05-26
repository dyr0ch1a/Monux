use std::path::PathBuf;


pub struct Config {
    pub notes_dir: PathBuf,
    pub daily_notes_dir: String,
    pub autosave: bool,
}


impl Config {
    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;


        let mut notes_dir = None;
        let mut daily_notes_dir = String::new();
        let mut autosave = true;


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
            } else if key.trim() == "daily_notes_dir" {
                daily_notes_dir =
value.trim().trim_matches('"').to_string();
            } else if key.trim() == "autosave" {
                let raw = value.trim();
                autosave = matches!(raw, "true" | "1" | "yes" | "on");
            }
        }


        Ok(Self {
            notes_dir: notes_dir.ok_or_else(||
anyhow::anyhow!("missing notes_dir"))?,
            daily_notes_dir,
            autosave,
        })
    }


    fn resolve_home(path: &str) -> PathBuf {
        if let Some((var, rest)) = Self::parse_env_path(path) {
            if let Ok(value) = std::env::var(var) {
                return Self::join_rest(PathBuf::from(value), rest);
            }
        }


        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                return home;
            }
            if let Some(p) = path.strip_prefix("~/") {
                return home.join(p);
            }
            if let Some(p) = path.strip_prefix("~\\") {
                return home.join(p);
            }
        }
        PathBuf::from(path)
    }


    fn parse_env_path(path: &str) -> Option<(&str, Option<&str>)> {
        let rest = path.strip_prefix("env(\"")?;
        let (var, tail) = rest.split_once("\")")?;
        if var.is_empty() {
            return None;
        }


        let tail = tail.strip_prefix('/').or_else(||
tail.strip_prefix('\\'));
        Some((var, tail))
    }


    fn join_rest(base: PathBuf, rest: Option<&str>) -> PathBuf {
        match rest {
            Some(suffix) if !suffix.is_empty() => base.join(suffix),
            _ => base,
        }
    }
}


