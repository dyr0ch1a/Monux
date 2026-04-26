use std::path::{Path, PathBuf};
use std::process::Command;

pub struct PluginRunReport {
    pub replacements: usize,
    pub errors: Vec<String>,
}

pub fn apply_plugins_in_file(path: &Path, plugins_dir: &Path) -> anyhow::Result<PluginRunReport> {
    let source = std::fs::read_to_string(path)?;
    let markers = parse_markers(&source);

    if markers.is_empty() {
        return Ok(PluginRunReport {
            replacements: 0,
            errors: Vec::new(),
        });
    }

    let mut output = String::with_capacity(source.len());
    let mut last = 0usize;
    let mut replacements = 0usize;
    let mut errors = Vec::new();

    for marker in markers {
        output.push_str(&source[last..marker.start]);
        match run_plugin(&marker.name, plugins_dir) {
            Ok(stdout) => {
                output.push_str(&stdout);
                replacements += 1;
            }
            Err(err) => {
                output.push_str(&source[marker.start..marker.end]);
                errors.push(format!("{}: {err}", marker.name));
            }
        }
        last = marker.end;
    }
    output.push_str(&source[last..]);

    if output != source {
        std::fs::write(path, output)?;
    }

    Ok(PluginRunReport {
        replacements,
        errors,
    })
}

struct Marker {
    start: usize,
    end: usize,
    name: String,
}

fn parse_markers(input: &str) -> Vec<Marker> {
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < input.len() {
        let Some(open_rel) = input[i..].find("_$$") else {
            break;
        };
        let open = i + open_rel;
        let inner_start = open + 3;

        let Some(close_rel) = input[inner_start..].find("$$_") else {
            break;
        };
        let close = inner_start + close_rel;
        let end = close + 3;

        let raw_name = input[inner_start..close].trim();
        if is_valid_plugin_name(raw_name) {
            out.push(Marker {
                start: open,
                end,
                name: raw_name.to_string(),
            });
        }

        i = end;
    }

    out
}

fn is_valid_plugin_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn run_plugin(name: &str, plugins_dir: &Path) -> anyhow::Result<String> {
    let wasm_path = plugin_path(plugins_dir, name);
    if !wasm_path.exists() {
        anyhow::bail!("plugin wasm not found: {}", wasm_path.display());
    }

    let wasmtime = std::env::var("MONUX_WASMTIME").unwrap_or_else(|_| "wasmtime".to_string());
    let output = Command::new(wasmtime).arg(&wasm_path).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            anyhow::bail!("plugin exited with {}", output.status);
        }
        anyhow::bail!("{stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn plugin_path(plugins_dir: &Path, name: &str) -> PathBuf {
    plugins_dir.join(format!("{name}.wasm"))
}
