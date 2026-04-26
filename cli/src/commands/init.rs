use crate::commands::context::CommandContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    ctx.init()?;
    let config = ctx.config_path();
    let plugins_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("No home directory"))?
        .join(".monux/plugins");

    if !config.exists() {
        std::fs::write(
            config,
            "notes_dir = \"~/notes\"\nplugins_dir = \"~/.monux/plugins\"\n",
        )?;
    }

    std::fs::create_dir_all(plugins_dir)?;
    Ok(())
}
