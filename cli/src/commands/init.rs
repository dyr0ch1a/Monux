use crate::commands::context::CommandContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    ctx.init()?;
    let config = ctx.config_path();
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
    let notes_dir = dirs::document_dir()
        .or_else(|| Some(home.join("notes")))
        .ok_or_else(|| anyhow::anyhow!("No documents/home directory"))?;

    if !config.exists() {
        let content = format!("notes_dir = \"{}\"\n", notes_dir.display());
        std::fs::write(config, content)?;
    }

    let cfg = ctx.load_config()?;
    std::fs::create_dir_all(&cfg.notes_dir)?;

    println!("config: {}", ctx.config_path().display());
    println!("notes_dir: {}", cfg.notes_dir.display());
    Ok(())
}
