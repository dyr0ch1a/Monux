use crate::commands::context::CommandContext;


pub fn run() -> anyhow::Result<()> {
    let ctx = CommandContext::new()?;
    ctx.init()?;
    let config = ctx.config_path();
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home
directory"))?;
    let notes_dir = default_notes_dir(&home);


    if !config.exists() {
        let content = format!(
            "notes_dir = \"{}\"\ndaily_notes_dir = \"daily\"\nautosave
= true\n",
            notes_dir.display()
        );
        std::fs::write(config, content)?;
    }


    let cfg = ctx.load_config()?;
    std::fs::create_dir_all(&cfg.notes_dir)?;


    println!("config: {}", ctx.config_path().display());
    println!("notes_dir: {}", cfg.notes_dir.display());
    Ok(())
}

#[cfg(target_os = "windows")]
fn default_notes_dir(home: &std::path::Path) -> std::path::PathBuf {
    home.join("Document").join("notes")
}

#[cfg(not(target_os = "windows"))]
fn default_notes_dir(home: &std::path::Path) -> std::path::PathBuf {
    dirs::document_dir()
        .map(|docs| docs.join("notes"))
        .unwrap_or_else(|| home.join("notes"))
}

