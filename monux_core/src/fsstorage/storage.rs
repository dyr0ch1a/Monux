use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::index::{normalize_note_path, parse_tags_input, path_key};

pub struct NoteStorage {
    notes_root: PathBuf,
}

impl NoteStorage {
    pub fn new(notes_root: PathBuf) -> Self {
        Self { notes_root }
    }

    pub fn notes_root(&self) -> &Path {
        &self.notes_root
    }

    pub fn list_note_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        if !self.notes_root.exists() {
            return Ok(out);
        }
        for entry in WalkDir::new(&self.notes_root) {
            let entry = entry?;
            let abs = entry.path();
            if !abs.is_file() || abs.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Ok(rel) = abs.strip_prefix(&self.notes_root) else {
                continue;
            };
            out.push(rel.to_path_buf());
        }
        out.sort_by_key(|p| path_key(p));
        Ok(out)
    }

    pub fn create_note(&self, rel: &Path, content: &str) -> anyhow::Result<()> {
        let abs = self.notes_root.join(rel);
        if abs.exists() {
            anyhow::bail!("note '{}' already exists", path_key(rel));
        }
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(abs, content)?;
        Ok(())
    }

    pub fn rename_note(&self, old_rel: &Path, new_rel: &Path) -> anyhow::Result<()> {
        let old_abs = self.notes_root.join(old_rel);
        let new_abs = self.notes_root.join(new_rel);
        if !old_abs.exists() {
            anyhow::bail!("note '{}' not found on disk", path_key(old_rel));
        }
        if let Some(parent) = new_abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(old_abs, new_abs)?;
        Ok(())
    }

    pub fn delete_note(&self, rel: &Path) -> anyhow::Result<()> {
        let abs = self.notes_root.join(rel);
        match std::fs::remove_file(abs) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn read_tags(&self, rel: &Path) -> anyhow::Result<Vec<String>> {
        let abs = self.notes_root.join(rel);
        let content = std::fs::read_to_string(abs).unwrap_or_default();
        Ok(read_tags_from_markdown(&content))
    }

    pub fn write_tags(&self, rel: &Path, tags: &[String]) -> anyhow::Result<Vec<String>> {
        let abs = self.notes_root.join(rel);
        let current = std::fs::read_to_string(&abs).unwrap_or_default();
        let normalized = normalize_tags(tags);
        let next = write_tags_to_markdown(&current, &normalized);
        std::fs::write(abs, next)?;
        Ok(normalized)
    }
}

pub fn tags_frontmatter(tags: &[String]) -> String {
    let normalized = normalize_tags(tags);
    if normalized.is_empty() {
        String::new()
    } else {
        format!("---\ntags: [{}]\n---\n", normalized.join(", "))
    }
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let raw = tags.join(" ");
    parse_tags_input(&raw)
}

fn write_tags_to_markdown(content: &str, tags: &[String]) -> String {
    let normalized = normalize_tags(tags);
    let (existing_frontmatter, body) = split_frontmatter(content)
        .map(|(fm, body)| (Some(fm), body))
        .unwrap_or_else(|| (None, content.replace("\r\n", "\n")));

    let mut lines = existing_frontmatter
        .map(|fm| fm.lines().map(ToString::to_string).collect::<Vec<String>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|line| !line.trim().starts_with("tags:"))
        .collect::<Vec<_>>();

    if !normalized.is_empty() {
        lines.push(format!("tags: [{}]", normalized.join(", ")));
    }

    if lines.is_empty() {
        return body;
    }

    format!("---\n{}\n---\n{}", lines.join("\n"), body)
}

fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let normalized = content.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    if lines.next()? != "---" {
        return None;
    }

    let mut frontmatter = Vec::new();
    let mut in_frontmatter = true;
    let mut body = Vec::new();
    for line in lines {
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            continue;
        }
        if in_frontmatter {
            frontmatter.push(line.to_string());
        } else {
            body.push(line.to_string());
        }
    }

    if in_frontmatter {
        return None;
    }

    let body_text = if body.is_empty() {
        String::new()
    } else {
        format!("{}\n", body.join("\n"))
    };
    Some((frontmatter.join("\n"), body_text))
}

fn read_tags_from_markdown(content: &str) -> Vec<String> {
    let Some((frontmatter, _body)) = split_frontmatter(content) else {
        return Vec::new();
    };

    for line in frontmatter.lines() {
        let trimmed = line.trim();
        let Some(raw) = trimmed.strip_prefix("tags:") else {
            continue;
        };
        return parse_tags_frontmatter_value(raw.trim());
    }
    Vec::new()
}

fn parse_tags_frontmatter_value(raw: &str) -> Vec<String> {
    let value = raw.trim();
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or(value);
    let tokens = inner
        .split(',')
        .flat_map(|chunk| chunk.split_whitespace())
        .map(|tag| tag.trim().trim_matches('"').trim_matches('\'').to_string())
        .collect::<Vec<_>>();
    parse_tags_input(&tokens.join(" "))
}

pub fn normalize_rel(raw: &str) -> PathBuf {
    normalize_note_path(raw)
}
