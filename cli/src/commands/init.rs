use crate::commands::context::CommandContext;

pub fn run() -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    ctx.init()?;
    let config = ctx.config_path();

    if !config.exists() {
        std::fs::write(config, r#"notes_dir = "~/notes""#)?;
    }
    Ok(())
}
