use core::storage::app_dir::AppDir;
use core::storage::config::Config;

pub fn run(name: Option<String>) -> anyhow::Result<()> {
    let app_dir = AppDir::new()?;
    let config_path = AppDir::config_path(&app_dir);
    let config = Config::load(config_path)?;

    std::fs::create_dir_all(&config.notes_dir)?;

    let output_path = match name {
        Some(name) => {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                anyhow::bail!("note name cannot be empty");
            }
            let name = format!("{}.md", trimmed);
            let path = config.notes_dir.join(name);
            if !path.exists() {
                std::fs::File::create(&path)?;
            }
            path
        }
        None => config.notes_dir.clone(),
    };

    println!("{}", output_path.display());
    Ok(())
}
