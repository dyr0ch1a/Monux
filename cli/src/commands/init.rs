pub fn run() -> anyhow::Result<()> {
    create_app_dir()?;
    create_placeholders()?;
    println!("Monux initialized");
    Ok(())
}

fn app_dir() -> anyhow::Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory found"))?;

    Ok(home.join(".monux"))
}
fn create_app_dir() -> anyhow::Result<()> {
    let path = app_dir()?;

    std::fs::create_dir_all(&path)?;

    Ok(())
}

fn create_placeholders() -> anyhow::Result<()> {
    let dir = app_dir()?;

    std::fs::write(dir.join("config.toml"), "")?;
    std::fs::write(dir.join("db.sqlite"), "")?;

    Ok(())
}
