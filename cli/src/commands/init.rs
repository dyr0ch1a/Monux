use core::storage::app_dir::AppDir;

pub fn run() -> anyhow::Result<()> {
    let app_dir = AppDir::new()?;
    app_dir.init()?;
    let config = app_dir.config_path();

    if !config.exists() {
        std::fs::write(config, r#"notes_dir = "~/notes""#)?;
    }
    Ok(())
}
