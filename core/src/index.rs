use std::path::{Path, PathBuf};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

const NOTES_BY_ID: TableDefinition<u64, &str> = TableDefinition::new("notes_by_id");
const TITLES_BY_ID: TableDefinition<u64, &str> = TableDefinition::new("titles_by_id");
const SLUG_TO_ID: TableDefinition<&str, u64> = TableDefinition::new("slug_to_id");
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");

#[derive(Debug, Clone)]
pub struct NoteMeta {
    pub id: u64,
    pub slug: String,
    pub title: String,
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

        let slug = slugify(title);
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
            let next_id = meta
                .get("next_id")?
                .map(|value| value.value())
                .unwrap_or(1);

            let mut notes = write_txn.open_table(NOTES_BY_ID)?;
            let mut titles = write_txn.open_table(TITLES_BY_ID)?;

            notes.insert(&next_id, slug.as_str())?;
            titles.insert(&next_id, title)?;
            slug_to_id.insert(slug.as_str(), &next_id)?;
            meta.insert("next_id", &(next_id + 1))?;
        }
        write_txn.commit()?;

        Ok(NoteMeta {
            id: self.id_by_slug(&slug)?.ok_or_else(|| anyhow::anyhow!("failed to fetch note id"))?,
            slug,
            title: title.to_string(),
        })
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

    pub fn get_by_slug(&self, raw_slug: &str) -> anyhow::Result<Option<NoteMeta>> {
        let slug = normalize_slug(raw_slug);
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

            let slug = notes.get(&id)?.map(|value| value.value().to_string());
            if let Some(slug) = slug {
                notes.remove(&id)?;
                titles.remove(&id)?;
                slug_to_id.remove(slug.as_str())?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    fn init_meta(&self) -> anyhow::Result<()> {
        let write_txn = self.db.begin_write()?;
        {
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

    let last_component = std::path::Path::new(raw)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(raw);

    let no_ext = last_component
        .strip_suffix(".md")
        .unwrap_or(last_component);

    slugify(no_ext)
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
