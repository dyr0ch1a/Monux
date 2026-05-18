use std::path::{Path, PathBuf};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, TableError};

const NOTES_BY_ID: TableDefinition<u64, &str> = TableDefinition::new("notes_by_id");
const TITLES_BY_ID: TableDefinition<u64, &str> = TableDefinition::new("titles_by_id");
const SLUG_TO_ID: TableDefinition<&str, u64> = TableDefinition::new("slug_to_id");
const TAGS_BY_ID: TableDefinition<u64, &str> = TableDefinition::new("tags_by_id");
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");

#[derive(Debug, Clone)]
pub struct NoteMeta {
    pub id: u64,
    pub slug: String,
    pub title: String,
}

fn normalize_note_key(raw: &str) -> String {
    let unified = raw.trim().replace('\\', "/");
    let no_ext = unified.strip_suffix(".md").unwrap_or(&unified);
    let parts = no_ext
        .split('/')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(slugify)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    parts.join("/")
}

pub struct NoteIndex {
    db: Database,
}

impl NoteIndex {
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Database::create(path)?;
        let this = Self { db };
        this.init_meta()?;
        Ok(this)
    }

    pub fn create_note(&self, name: &str) -> anyhow::Result<NoteMeta> {
        let title = name.trim();
        if title.is_empty() {
            anyhow::bail!("note name cannot be empty");
        }

        let slug = normalize_note_key(title);
        if slug.is_empty() {
            anyhow::bail!("failed to build note slug from name");
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut slug_to_id = write_txn.open_table(SLUG_TO_ID)?;
            if slug_to_id.get(slug.as_str())?.is_some() {
                anyhow::bail!("note '{}' already exists", slug);
            }

            let mut meta = write_txn.open_table(META)?;
            let next_id = meta.get("next_id")?.map(|value| value.value()).unwrap_or(1);

            let mut notes = write_txn.open_table(NOTES_BY_ID)?;
            let mut titles = write_txn.open_table(TITLES_BY_ID)?;

            notes.insert(&next_id, slug.as_str())?;
            titles.insert(&next_id, title)?;
            slug_to_id.insert(slug.as_str(), &next_id)?;
            meta.insert("next_id", &(next_id + 1))?;
        }
        write_txn.commit()?;

        Ok(NoteMeta {
            id: self
                .id_by_slug(&slug)?
                .ok_or_else(|| anyhow::anyhow!("failed to fetch note id"))?,
            slug,
            title: title.to_string(),
        })
    }

    pub fn create_note_with_tags(&self, name: &str, tags: &[String]) -> anyhow::Result<NoteMeta> {
        let note = self.create_note(name)?;
        self.add_tags(note.id, tags)?;
        Ok(note)
    }

    pub fn list(&self) -> anyhow::Result<Vec<NoteMeta>> {
        let read_txn = self.db.begin_read()?;
        let notes = read_txn.open_table(NOTES_BY_ID)?;
        let titles = read_txn.open_table(TITLES_BY_ID)?;

        let mut out = Vec::new();
        for entry in notes.iter()? {
            let (id_guard, slug_guard) = entry?;
            let id = id_guard.value();
            let slug = slug_guard.value().to_string();
            let title = titles
                .get(&id)?
                .map(|value| value.value().to_string())
                .unwrap_or_else(|| slug.clone());

            out.push(NoteMeta { id, slug, title });
        }

        out.sort_by_key(|n| n.id);
        Ok(out)
    }

    pub fn find(&self, query: &str) -> anyhow::Result<Vec<NoteMeta>> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let notes = self.list()?;
        Ok(notes
            .into_iter()
            .filter(|note| {
                note.slug.to_lowercase().contains(&query)
                    || note.title.to_lowercase().contains(&query)
            })
            .collect())
    }

    pub fn find_by_tag(&self, tag: &str) -> anyhow::Result<Vec<NoteMeta>> {
        let wanted = normalize_tag(tag);
        if wanted.is_empty() {
            return Ok(Vec::new());
        }

        let notes = self.list()?;
        let mut out = Vec::new();
        for note in notes {
            let tags = self.list_tags(note.id)?;
            if tags.iter().any(|t| t == &wanted) {
                out.push(note);
            }
        }
        Ok(out)
    }

    pub fn add_tags(&self, note_id: u64, tags: &[String]) -> anyhow::Result<Vec<String>> {
        let mut merged = self.list_tags(note_id)?;
        merged.extend(tags.iter().cloned());
        merged = normalize_tags(&merged);

        let encoded = encode_tags(&merged);
        let write_txn = self.db.begin_write()?;
        {
            let notes = write_txn.open_table(NOTES_BY_ID)?;
            if notes.get(&note_id)?.is_none() {
                anyhow::bail!("note id '{}' not found", note_id);
            }

            let mut table = write_txn.open_table(TAGS_BY_ID)?;
            table.insert(&note_id, encoded.as_str())?;
        }
        write_txn.commit()?;
        Ok(merged)
    }

    pub fn add_tags_to_slug(&self, slug: &str, tags: &[String]) -> anyhow::Result<Vec<String>> {
        let normalized = normalize_note_key(slug);
        let id = self
            .id_by_slug(&normalized)?
            .ok_or_else(|| anyhow::anyhow!("note '{}' not found", normalized))?;
        self.add_tags(id, tags)
    }

    pub fn list_tags(&self, note_id: u64) -> anyhow::Result<Vec<String>> {
        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(TAGS_BY_ID) {
            Ok(table) => table,
            Err(TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };
        let raw = table
            .get(&note_id)?
            .map(|value| value.value().to_string())
            .unwrap_or_default();
        Ok(decode_tags(&raw))
    }

    pub fn list_tags_by_slug(&self, slug: &str) -> anyhow::Result<Vec<String>> {
        let normalized = normalize_note_key(slug);
        let id = self
            .id_by_slug(&normalized)?
            .ok_or_else(|| anyhow::anyhow!("note '{}' not found", normalized))?;
        self.list_tags(id)
    }

    pub fn get_by_slug(&self, raw_slug: &str) -> anyhow::Result<Option<NoteMeta>> {
        let slug = normalize_note_key(raw_slug);
        if slug.is_empty() {
            return Ok(None);
        }

        let id = match self.id_by_slug(&slug)? {
            Some(id) => id,
            None => return Ok(None),
        };

        let read_txn = self.db.begin_read()?;
        let notes = read_txn.open_table(NOTES_BY_ID)?;
        let titles = read_txn.open_table(TITLES_BY_ID)?;

        let note_slug = notes
            .get(&id)?
            .map(|value| value.value().to_string())
            .ok_or_else(|| anyhow::anyhow!("note index corrupted: missing notes_by_id entry"))?;

        let title = titles
            .get(&id)?
            .map(|value| value.value().to_string())
            .unwrap_or_else(|| note_slug.clone());

        Ok(Some(NoteMeta {
            id,
            slug: note_slug,
            title,
        }))
    }

    pub fn delete(&self, id: u64) -> anyhow::Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut notes = write_txn.open_table(NOTES_BY_ID)?;
            let mut titles = write_txn.open_table(TITLES_BY_ID)?;
            let mut slug_to_id = write_txn.open_table(SLUG_TO_ID)?;
            let mut tags = write_txn.open_table(TAGS_BY_ID)?;

            let slug = notes.get(&id)?.map(|value| value.value().to_string());
            if let Some(slug) = slug {
                notes.remove(&id)?;
                titles.remove(&id)?;
                slug_to_id.remove(slug.as_str())?;
                tags.remove(&id)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn rename(&self, old_slug: &str, new_name: &str) -> anyhow::Result<NoteMeta> {
        let current = self
            .get_by_slug(old_slug)?
            .ok_or_else(|| anyhow::anyhow!("note '{}' not found", normalize_note_key(old_slug)))?;

        let title = new_name.trim();
        if title.is_empty() {
            anyhow::bail!("new note name cannot be empty");
        }

        let new_slug = normalize_note_key(title);
        if new_slug.is_empty() {
            anyhow::bail!("failed to build note slug from new name");
        }

        if new_slug != current.slug {
            if self.get_by_slug(&new_slug)?.is_some() {
                anyhow::bail!("note '{}' already exists", new_slug);
            }
        }

        let write_txn = self.db.begin_write()?;
        {
            let mut notes = write_txn.open_table(NOTES_BY_ID)?;
            let mut titles = write_txn.open_table(TITLES_BY_ID)?;
            let mut slug_to_id = write_txn.open_table(SLUG_TO_ID)?;

            notes.insert(&current.id, new_slug.as_str())?;
            titles.insert(&current.id, title)?;
            slug_to_id.remove(current.slug.as_str())?;
            slug_to_id.insert(new_slug.as_str(), &current.id)?;
        }
        write_txn.commit()?;

        Ok(NoteMeta {
            id: current.id,
            slug: new_slug,
            title: title.to_string(),
        })
    }

    fn init_meta(&self) -> anyhow::Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            // Schema migration guard for old DB files.
            let _ = write_txn.open_table(NOTES_BY_ID)?;
            let _ = write_txn.open_table(TITLES_BY_ID)?;
            let _ = write_txn.open_table(SLUG_TO_ID)?;
            let _ = write_txn.open_table(TAGS_BY_ID)?;
            let mut meta = write_txn.open_table(META)?;
            if meta.get("next_id")?.is_none() {
                meta.insert("next_id", &1)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    fn id_by_slug(&self, slug: &str) -> anyhow::Result<Option<u64>> {
        let read_txn = self.db.begin_read()?;
        let slug_to_id = read_txn.open_table(SLUG_TO_ID)?;
        Ok(slug_to_id.get(slug)?.map(|value| value.value()))
    }
}

pub fn note_path(notes_root: &Path, slug: &str) -> PathBuf {
    notes_root.join(format!("{slug}.md"))
}

pub fn normalize_slug(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    let raw = raw.split_once('|').map(|(target, _)| target).unwrap_or(raw);
    normalize_note_key(raw)
}

pub fn note_slug_with_dir(dir: Option<&str>, name: &str) -> String {
    let name_slug = normalize_note_key(name);
    if name_slug.is_empty() {
        return String::new();
    }

    let dir_slug = dir.map(normalize_note_key).unwrap_or_default();
    if dir_slug.is_empty() {
        name_slug
    } else {
        format!("{dir_slug}/{name_slug}")
    }
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;

    for ch in input.trim().chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch == '-' || ch == '_' {
            Some(ch)
        } else if ch.is_whitespace() {
            Some('-')
        } else {
            None
        };

        if let Some(ch) = mapped {
            if ch == '-' {
                if !prev_dash {
                    slug.push('-');
                    prev_dash = true;
                }
            } else {
                slug.push(ch);
                prev_dash = false;
            }
        }
    }

    slug.trim_matches('-').to_string()
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

    #[test]
    fn normalize_slug_handles_links_paths_and_ext() {
        assert_eq!(normalize_slug("Some Note"), "some-note");
        assert_eq!(normalize_slug("folder/Some Note.md"), "folder/some-note");
        assert_eq!(normalize_slug("[[Some Note|Alias]]"), "some-note");
    }

    #[test]
    fn parse_tags_normalizes_and_deduplicates() {
        let tags = parse_tags_input(" Rust, rust  CLI #ignored cli ");
        assert_eq!(tags, vec!["rust", "cli", "ignored"]);
    }

    #[test]
    fn create_find_tags_delete_workflow() -> anyhow::Result<()> {
        let path = temp_db_path("workflow");
        let index = NoteIndex::open(path.clone())?;

        let first = index.create_note_with_tags("First Note", &["rust".to_string()])?;
        let second = index.create_note("Second Note")?;

        let found = index.find("first")?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, first.id);

        let by_tag = index.find_by_tag("rust")?;
        assert_eq!(by_tag.len(), 1);
        assert_eq!(by_tag[0].id, first.id);

        index.delete(first.id)?;
        assert!(index.get_by_slug("first-note")?.is_none());
        assert!(index.get_by_slug("second-note")?.is_some());
        assert_eq!(index.list()?.len(), 1);
        assert_eq!(index.list()?[0].id, second.id);

        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn rename_updates_slug_and_title() -> anyhow::Result<()> {
        let path = temp_db_path("rename");
        let index = NoteIndex::open(path.clone())?;

        let created = index.create_note("Old Name")?;
        let renamed = index.rename("old-name", "New Name")?;

        assert_eq!(renamed.id, created.id);
        assert_eq!(renamed.slug, "new-name");
        assert_eq!(renamed.title, "New Name");
        assert!(index.get_by_slug("old-name")?.is_none());
        assert!(index.get_by_slug("new-name")?.is_some());

        let _ = std::fs::remove_file(path);
        Ok(())
    }
}
