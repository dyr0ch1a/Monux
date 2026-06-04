use std::path::{Path, PathBuf};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, TableError};
use walkdir::WalkDir;

const TAGS_BY_PATH: TableDefinition<&str, &str> = TableDefinition::new("tags_by_path");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteMeta {
    pub path: PathBuf,
    pub title: String,
}

impl NoteMeta {
    pub fn abs_path(&self, notes_root: &Path) -> PathBuf {
        notes_root.join(&self.path)
    }

    pub fn display_path(&self) -> String {
        path_key(&self.path)
    }
}

pub struct NoteIndex {
    db: Database,
    notes_root: PathBuf,
}

impl NoteIndex {
    pub fn open(db_path: PathBuf, notes_root: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Database::create(db_path)?;
        let this = Self { db, notes_root };
        this.init_schema()?;
        Ok(this)
    }

    pub fn notes_root(&self) -> &Path {
        &self.notes_root
    }

    pub fn create_note(&self, name: &str) -> anyhow::Result<NoteMeta> {
        let rel = normalize_note_path(name);
        if rel.as_os_str().is_empty() {
            anyhow::bail!("failed to build note path from name");
        }

        let abs = self.notes_root.join(&rel);
        if abs.exists() {
            anyhow::bail!("note '{}' already exists", path_key(&rel));
        }

        Ok(self.note_meta_for_rel(rel, Some(name.trim()))?)
    }

    pub fn create_note_with_tags(&self, name: &str, tags: &[String]) -> anyhow::Result<NoteMeta> {
        let note = self.create_note(name)?;
        let abs = self.notes_root.join(&note.path);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !abs.exists() {
            std::fs::File::create(&abs)?;
        }
        let current = std::fs::read_to_string(&abs).unwrap_or_default();
        let next = write_tags_to_markdown(&current, tags);
        std::fs::write(&abs, next)?;
        self.reindex_note(&note.path)?;
        Ok(note)
    }

    pub fn list(&self) -> anyhow::Result<Vec<NoteMeta>> {
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
            let rel = rel.to_path_buf();
            if rel.as_os_str().is_empty() {
                continue;
            }

            out.push(self.note_meta_for_rel(rel, None)?);
        }

        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    pub fn find(&self, query: &str) -> anyhow::Result<Vec<NoteMeta>> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored = self
            .list()?
            .into_iter()
            .filter_map(|note| {
                let title_score = similarity_score(&query, &note.title);
                let path_score = similarity_score(&query, &note.display_path());
                let score = title_score.max(path_score);
                if score >= 0.45 {
                    Some((score, note))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.path.cmp(&b.1.path))
        });
        Ok(scored.into_iter().map(|(_, note)| note).collect())
    }

    pub fn find_by_tag(&self, tag: &str) -> anyhow::Result<Vec<NoteMeta>> {
        let wanted = normalize_tag(tag);
        if wanted.is_empty() {
            return Ok(Vec::new());
        }

        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(TAGS_BY_PATH) {
            Ok(table) => table,
            Err(TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };

        let mut out = Vec::new();
        for entry in table.iter()? {
            let (path_guard, encoded_guard) = entry?;
            let tags = decode_tags(encoded_guard.value());
            if !tags.iter().any(|t| t == &wanted) {
                continue;
            }

            let rel = resolve_note_path(path_guard.value());
            if let Some(note) = self.get_path(&rel)? {
                out.push(note);
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    pub fn add_tags(&self, path: impl AsRef<Path>, tags: &[String]) -> anyhow::Result<Vec<String>> {
        let rel = self.require_note_path(path.as_ref())?;
        let mut merged = self.list_tags(&rel)?;
        merged.extend(tags.iter().cloned());
        merged = normalize_tags(&merged);
        self.store_tags(&rel, &merged)?;
        Ok(merged)
    }

    pub fn set_tags(&self, path: impl AsRef<Path>, tags: &[String]) -> anyhow::Result<Vec<String>> {
        let rel = self.require_note_path(path.as_ref())?;
        let normalized = normalize_tags(tags);
        self.store_tags(&rel, &normalized)?;
        Ok(normalized)
    }

    pub fn list_tags(&self, path: impl AsRef<Path>) -> anyhow::Result<Vec<String>> {
        let rel = self.require_note_path(path.as_ref())?;
        let abs = self.notes_root.join(&rel);
        let content = std::fs::read_to_string(abs).unwrap_or_default();
        Ok(read_tags_from_markdown(&content))
    }

    pub fn get(&self, raw: &str) -> anyhow::Result<Option<NoteMeta>> {
        let rel = resolve_note_path(raw);
        self.get_path(&rel)
    }

    pub fn get_path(&self, rel: &Path) -> anyhow::Result<Option<NoteMeta>> {
        let rel = rel.to_path_buf();
        if rel.as_os_str().is_empty() {
            return Ok(None);
        }

        let abs = self.notes_root.join(&rel);
        if !abs.is_file() {
            return Ok(None);
        }

        Ok(Some(self.note_meta_for_rel(rel, None)?))
    }

    pub fn delete(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let rel = resolve_note_path_ref(path.as_ref());
        if rel.as_os_str().is_empty() {
            return Ok(());
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TAGS_BY_PATH)?;
            table.remove(path_key(&rel).as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn delete_note(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let rel = resolve_note_path_ref(path.as_ref());
        if rel.as_os_str().is_empty() {
            return Ok(());
        }

        let abs = self.notes_root.join(&rel);
        match std::fs::remove_file(&abs) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        self.delete(&rel)?;
        let _ = self.prune_orphan_tags()?;
        Ok(())
    }

    pub fn reindex_note(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let rel = resolve_note_path_ref(path.as_ref());
        if rel.as_os_str().is_empty() {
            return Ok(());
        }

        let abs = self.notes_root.join(&rel);
        let tags = if abs.is_file() {
            let content = std::fs::read_to_string(abs).unwrap_or_default();
            read_tags_from_markdown(&content)
        } else {
            Vec::new()
        };

        let key = path_key(&rel);
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TAGS_BY_PATH)?;
            if tags.is_empty() {
                table.remove(key.as_str())?;
            } else {
                let encoded = encode_tags(&tags);
                table.insert(key.as_str(), encoded.as_str())?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn reindex_all(&self) -> anyhow::Result<usize> {
        let mut reindexed = 0usize;
        if self.notes_root.exists() {
            for entry in WalkDir::new(&self.notes_root) {
                let entry = entry?;
                let abs = entry.path();
                if !abs.is_file() || abs.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Ok(rel) = abs.strip_prefix(&self.notes_root) else {
                    continue;
                };
                self.reindex_note(rel)?;
                reindexed += 1;
            }
        }
        let _ = self.prune_orphan_tags()?;
        Ok(reindexed)
    }

    pub fn rename(&self, old: &Path, new: &Path) -> anyhow::Result<()> {
        self.move_path(old, new)
    }

    pub fn rename_note(&self, old: impl AsRef<Path>, new_name: &str) -> anyhow::Result<NoteMeta> {
        let old_rel = resolve_note_path_ref(old.as_ref());
        if old_rel.as_os_str().is_empty() {
            anyhow::bail!("note path is invalid");
        }
        let new_rel = normalize_note_path(new_name);
        if new_rel.as_os_str().is_empty() {
            anyhow::bail!("new note name cannot be empty");
        }
        self.rename(&old_rel, &new_rel)?;
        self.note_meta_for_rel(new_rel, Some(new_name.trim()))
    }

    pub fn move_note(&self, old: impl AsRef<Path>, new: impl AsRef<Path>) -> anyhow::Result<()> {
        let old_rel = resolve_note_path_ref(old.as_ref());
        let new_rel = resolve_note_path_ref(new.as_ref());
        self.rename(&old_rel, &new_rel)
    }

    pub fn move_path(&self, old: &Path, new: &Path) -> anyhow::Result<()> {
        let old_rel = self.require_note_path(old)?;
        let new_rel = resolve_note_path_ref(new);
        if new_rel.as_os_str().is_empty() {
            anyhow::bail!("target note path is invalid");
        }
        if old_rel == new_rel {
            return Ok(());
        }
        if self.get_path(&new_rel)?.is_some() {
            anyhow::bail!("note '{}' already exists", path_key(&new_rel));
        }

        let old_abs = self.notes_root.join(&old_rel);
        let new_abs = self.notes_root.join(&new_rel);
        if let Some(parent) = new_abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(old_abs, new_abs)?;
        self.rewrite_prefix(&old_rel, &new_rel)
    }

    pub fn rename_dir(&self, old_raw: &str, new_raw: &str) -> anyhow::Result<usize> {
        self.move_dir(old_raw, new_raw)
    }

    pub fn move_dir(&self, old_raw: &str, new_raw: &str) -> anyhow::Result<usize> {
        let old_dir = normalize_dir_path(old_raw);
        let new_dir = normalize_dir_path(new_raw);
        if old_dir.is_empty() || new_dir.is_empty() {
            anyhow::bail!("directory path cannot be empty");
        }
        if old_dir == new_dir {
            return Ok(0);
        }
        if new_dir.starts_with(&(old_dir.clone() + "/")) {
            anyhow::bail!(
                "cannot rename directory into its own
child"
            );
        }

        let old_fs = self.notes_root.join(&old_dir);
        let new_fs = self.notes_root.join(&new_dir);
        if !old_fs.is_dir() {
            anyhow::bail!("directory '{}' not found", old_dir);
        }
        if new_fs.exists() {
            anyhow::bail!("directory '{}' already exists", new_dir);
        }
        if let Some(parent) = new_fs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&old_fs, &new_fs)?;

        let old_prefix = PathBuf::from(&old_dir);
        let new_prefix = PathBuf::from(&new_dir);
        self.rewrite_prefix_recursive(&old_prefix, &new_prefix)
    }

    pub fn delete_dir(&self, raw_dir: &str) -> anyhow::Result<()> {
        let dir = normalize_dir_path(raw_dir);
        if dir.is_empty() {
            anyhow::bail!("cannot delete root directory");
        }

        let dir_prefix = format!("{dir}/");
        let has_notes_inside = self
            .list()?
            .iter()
            .any(|note| path_key(&note.path).starts_with(&dir_prefix));
        if has_notes_inside {
            anyhow::bail!("directory {dir}/ is not empty");
        }

        let fs_path = self.notes_root.join(&dir);
        match std::fs::remove_dir(&fs_path) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        let _ = self.prune_orphan_tags()?;
        Ok(())
    }

    pub fn prune_orphan_tags(&self) -> anyhow::Result<usize> {
        let notes = self.list()?;
        let live: std::collections::HashSet<String> =
            notes.iter().map(|n| path_key(&n.path)).collect();

        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(TAGS_BY_PATH) {
            Ok(table) => table,
            Err(TableError::TableDoesNotExist(_)) => return Ok(0),
            Err(err) => return Err(err.into()),
        };

        let mut orphans = Vec::new();
        for entry in table.iter()? {
            let (path_guard, _) = entry?;
            let key = path_guard.value();
            if !live.contains(key) {
                orphans.push(key.to_string());
            }
        }
        drop(read_txn);

        if orphans.is_empty() {
            return Ok(0);
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TAGS_BY_PATH)?;
            for key in &orphans {
                table.remove(key.as_str())?;
            }
        }
        write_txn.commit()?;
        Ok(orphans.len())
    }

    fn require_note_path(&self, path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let rel = resolve_note_path_ref(path.as_ref());
        if rel.as_os_str().is_empty() {
            anyhow::bail!("note path is invalid");
        }
        if self.get_path(&rel)?.is_none() {
            anyhow::bail!("note '{}' not found", path_key(&rel));
        }
        Ok(rel)
    }

    fn move_tags(&self, old: &Path, new: &Path) -> anyhow::Result<()> {
        let old_key = path_key(old);
        let new_key = path_key(new);
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TAGS_BY_PATH)?;
            let old_tags = table.get(old_key.as_str())?.map(|v| v.value().to_string());
            table.remove(old_key.as_str())?;
            if let Some(encoded) = old_tags {
                if !encoded.trim().is_empty() {
                    table.insert(new_key.as_str(), encoded.as_str())?;
                }
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    fn rewrite_prefix(&self, old_prefix: &Path, new_prefix: &Path) -> anyhow::Result<()> {
        if old_prefix == new_prefix {
            return Ok(());
        }
        let old_key = path_key(old_prefix);
        let new_key = path_key(new_prefix);

        if self.get_path(old_prefix)?.is_some() {
            self.move_tags(old_prefix, new_prefix)?;
            return Ok(());
        }

        let old_prefix_key = format!("{old_key}/");
        let new_prefix_key = format!("{new_key}/");
        let _ = self.rewrite_prefix_recursive_with_keys(&old_prefix_key, &new_prefix_key)?;
        Ok(())
    }

    fn rewrite_prefix_recursive(
        &self,
        old_prefix: &Path,
        new_prefix: &Path,
    ) -> anyhow::Result<usize> {
        let old_prefix_key = format!("{}/", path_key(old_prefix));
        let new_prefix_key = format!("{}/", path_key(new_prefix));
        self.rewrite_prefix_recursive_with_keys(&old_prefix_key, &new_prefix_key)
    }

    fn rewrite_prefix_recursive_with_keys(
        &self,
        old_prefix_key: &str,
        new_prefix_key: &str,
    ) -> anyhow::Result<usize> {
        let write_txn = self.db.begin_write()?;
        let moved;
        {
            let mut table = write_txn.open_table(TAGS_BY_PATH)?;
            let mut replacements = Vec::new();
            for entry in table.iter()? {
                let (key_guard, value_guard) = entry?;
                let key = key_guard.value();
                if let Some(rest) = key.strip_prefix(old_prefix_key) {
                    let new_key = format!("{new_prefix_key}{rest}");
                    replacements.push((key.to_string(), new_key, value_guard.value().to_string()));
                }
            }
            moved = replacements.len();
            for (old_key, new_key, encoded) in replacements {
                table.remove(old_key.as_str())?;
                if !encoded.trim().is_empty() {
                    table.insert(new_key.as_str(), encoded.as_str())?;
                }
            }
        }
        write_txn.commit()?;
        Ok(moved)
    }

    fn store_tags(&self, rel: &Path, tags: &[String]) -> anyhow::Result<()> {
        let abs = self.notes_root.join(rel);
        let current = std::fs::read_to_string(&abs).unwrap_or_default();
        let next = write_tags_to_markdown(&current, tags);
        std::fs::write(&abs, next)?;
        self.reindex_note(rel)
    }

    fn note_meta_for_rel(
        &self,
        rel: PathBuf,
        fallback_name: Option<&str>,
    ) -> anyhow::Result<NoteMeta> {
        let abs = self.notes_root.join(&rel);
        let title = if abs.is_file() {
            let content = std::fs::read_to_string(&abs).unwrap_or_default();
            note_title_from_content(&content).unwrap_or_else(|| title_from_path(&rel))
        } else {
            fallback_name
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| title_from_path(&rel))
        };

        Ok(NoteMeta { path: rel, title })
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let _ = write_txn.open_table(TAGS_BY_PATH)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

pub fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn resolve_note_path(raw: &str) -> PathBuf {
    normalize_note_path(raw)
}

pub fn resolve_note_path_ref(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        return PathBuf::new();
    }
    normalize_note_path(&path_key(path))
}

pub fn normalize_note_path(raw: &str) -> PathBuf {
    let raw = raw.trim();
    if raw.is_empty() {
        return PathBuf::new();
    }
    let raw = raw
        .strip_prefix("[[")
        .and_then(|v| v.strip_suffix("]]"))
        .unwrap_or(raw);
    let raw = raw
        .split_once('|')
        .map(|(target, _)| target)
        .unwrap_or(raw)
        .trim();
    let unified = raw.replace('\\', "/");

    let mut parts = Vec::new();
    for segment in unified.split('/') {
        let segment = segment.trim();
        if segment.is_empty() || segment == "." || segment == ".." {
            continue;
        }
        let clean = sanitize_path_segment(segment);
        if clean.is_empty() {
            continue;
        }
        parts.push(clean);
    }

    if parts.is_empty() {
        return PathBuf::new();
    }

    let last = parts.last_mut().expect("non-empty parts");
    if !last.ends_with(".md") {
        last.push_str(".md");
    }

    parts.iter().collect()
}

pub fn note_path_with_dir(dir: Option<&str>, name: &str) -> PathBuf {
    let name_path = normalize_note_path(name);
    if name_path.as_os_str().is_empty() {
        return PathBuf::new();
    }

    let Some(dir) = dir.map(normalize_dir_path).filter(|d| !d.is_empty()) else {
        return name_path;
    };

    PathBuf::from(dir).join(name_path)
}

pub fn abs_note_path(notes_root: &Path, rel: &Path) -> PathBuf {
    notes_root.join(rel)
}

pub fn path_in_dir(rel: &Path, dir_filter: &str) -> bool {
    let prefix = normalize_dir_path(dir_filter);
    if prefix.is_empty() {
        return true;
    }
    let key = path_key(rel);
    key == prefix || key.starts_with(&(prefix + "/"))
}

pub fn note_title_from_content(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if trimmed.starts_with("---") {
        let body = &trimmed[3..];
        if let Some(end) = body.find("\n---") {
            for line in body[..end].lines() {
                let line = line.trim();
                let Some(value) = line.strip_prefix("title:") else {
                    continue;
                };
                let title = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
    }

    None
}

fn title_from_path(rel: &Path) -> String {
    rel.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

pub fn today_date_path() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn normalize_dir_path(raw: &str) -> String {
    let unified = raw.trim().replace('\\', "/");
    let parts = unified
        .split('/')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter(|p| *p != "." && *p != "..")
        .map(sanitize_path_segment)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    parts.join("/")
}

fn sanitize_path_segment(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| {
            !matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') && !ch.is_control()
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn similarity_score(query: &str, raw: &str) -> f32 {
    let candidate = raw.trim().to_lowercase();
    if candidate.is_empty() {
        return 0.0;
    }
    if candidate == query {
        return 1.0;
    }
    if candidate.contains(query) {
        return 0.92;
    }

    let q_chars = query.chars().count();
    let c_chars = candidate.chars().count();
    let max_len = q_chars.max(c_chars);
    if max_len == 0 {
        return 0.0;
    }

    let dist = levenshtein(query, &candidate);
    let distance_score = 1.0 - (dist as f32 / max_len as f32);

    let q_words = query.split_whitespace().collect::<Vec<_>>();
    let token_overlap = if q_words.is_empty() {
        0.0
    } else {
        let hits = q_words
            .iter()
            .filter(|part| candidate.contains(**part))
            .count();
        hits as f32 / q_words.len() as f32
    };

    (distance_score * 0.7) + (token_overlap * 0.3)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars = b.chars().collect::<Vec<_>>();
    let mut prev = (0..=b_chars.len()).collect::<Vec<_>>();
    let mut curr = vec![0usize; b_chars.len() + 1];

    for (i, a_ch) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, b_ch) in b_chars.iter().enumerate() {
            let cost = usize::from(a_ch != *b_ch);
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_chars.len()]
}

pub fn parse_tags_input(raw: &str) -> Vec<String> {
    let tokens: Vec<String> = raw
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();
    normalize_tags(&tokens)
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let normalized = normalize_tag(tag);
        if !normalized.is_empty() && !out.iter().any(|existing| existing == &normalized) {
            out.push(normalized);
        }
    }
    out
}

fn normalize_tag(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

fn encode_tags(tags: &[String]) -> String {
    tags.join(",")
}

fn decode_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
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
    normalize_tags(&tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("monux-{name}-{nanos}.redb"))
    }

    fn temp_notes_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("monux-notes-{name}-{nanos}"))
    }

    #[test]
    fn normalize_note_path_handles_links_paths_and_ext() {
        assert_eq!(
            normalize_note_path("Some Note"),
            PathBuf::from("Some Note.md")
        );
        assert_eq!(
            normalize_note_path("folder/Some Note.md"),
            PathBuf::from("folder/Some Note.md")
        );
        assert_eq!(
            normalize_note_path("[[Some Note|Alias]]"),
            PathBuf::from("Some Note.md")
        );
        assert_eq!(
            normalize_note_path("vault/rust/ownership.md"),
            PathBuf::from("vault/rust/ownership.md")
        );
    }

    #[test]
    fn parse_tags_normalizes_and_deduplicates() {
        let tags = parse_tags_input(" Rust, rust  CLI #ignored cli ");
        assert_eq!(tags, vec!["rust", "cli", "ignored"]);
    }

    #[test]
    fn note_title_from_content_reads_frontmatter_only() {
        let fm = "---\ntitle: My Title\n---\n\n# Ignored\n";
        assert_eq!(note_title_from_content(fm).as_deref(), Some("My Title"));

        let heading = "# First Heading\nbody\n";
        assert_eq!(note_title_from_content(heading), None);
    }

    #[test]
    fn tags_find_delete_workflow() -> anyhow::Result<()> {
        let notes_root = temp_notes_root("workflow");
        std::fs::create_dir_all(&notes_root)?;
        let first_rel = PathBuf::from("First Note.md");
        std::fs::write(abs_note_path(&notes_root, &first_rel), "# First Note\n")?;
        std::fs::write(
            abs_note_path(&notes_root, &PathBuf::from("Second Note.md")),
            "# Second\n",
        )?;

        let path = temp_db_path("workflow");
        let index = NoteIndex::open(path.clone(), notes_root.clone())?;

        index.add_tags(&first_rel, &["rust".to_string()])?;

        let found = index.find("first")?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, first_rel);

        let by_tag = index.find_by_tag("rust")?;
        assert_eq!(by_tag.len(), 1);
        assert_eq!(by_tag[0].path, PathBuf::from("First Note.md"));

        std::fs::remove_file(abs_note_path(&notes_root, &first_rel))?;
        index.delete(&first_rel)?;
        assert!(index.get("First Note")?.is_none());
        assert!(index.get("Second Note")?.is_some());
        assert_eq!(index.list()?.len(), 1);

        let _ = std::fs::remove_dir_all(notes_root);
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn rename_moves_tags_to_new_path() -> anyhow::Result<()> {
        let notes_root = temp_notes_root("rename");
        std::fs::create_dir_all(&notes_root)?;
        let old_rel = PathBuf::from("Old Name.md");
        std::fs::write(abs_note_path(&notes_root, &old_rel), "# Old\n")?;

        let path = temp_db_path("rename");
        let index = NoteIndex::open(path.clone(), notes_root.clone())?;
        index.add_tags(&old_rel, &["todo".to_string()])?;

        let new_rel = PathBuf::from("New Name.md");
        index.rename(&old_rel, &new_rel)?;
        assert_eq!(index.list_tags(&new_rel)?, vec!["todo"]);
        assert!(index.get_path(&old_rel)?.is_none());

        let _ = std::fs::remove_dir_all(notes_root);
        let _ = std::fs::remove_file(path);
        Ok(())
    }
}
