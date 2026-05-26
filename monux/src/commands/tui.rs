use std::process::Command;

pub fn run() -> anyhow::Result<()> {
    match Command::new("monux_tui").status() {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else {
                anyhow::bail!("monux_tui exited with status: {status}");
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "monux_tui binary was not found in PATH. Install it with: cargo install --path monux_tui"
            );
        }
        Err(err) => Err(err.into()),
    }
}
