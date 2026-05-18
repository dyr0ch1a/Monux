use std::path::PathBuf;
use std::string::String;
use walkdir::WalkDir;

pub struct Notes {
    root: PathBuf,
}

impl Notes {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn list(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut notes = Vec::new();
        for entry in WalkDir::new(&self.root) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                notes.push(path.to_path_buf());
            }
        }
        Ok(notes)
    }

    pub fn find(&self, query: String) -> anyhow::Result<Vec<PathBuf>> {
        let notes = self.list()?;

        let result = notes
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| name.contains(&query))
                    .unwrap_or(false)
            })
            .collect();

        Ok(result)
    }
    pub fn delete(&self, query: String) -> anyhow::Result<()> {
        let files = self.find(query)?;

        if files.is_empty() {
            println!("Nothing found");
            return Ok(());
        }

        for file in files {
            println!("Deleting: {}", file.display());
            std::fs::remove_file(file)?;
        }
        Ok(())
    }
}
