use monux_core::context::StorageContext;
use monux_core::fsstorage::config::Config;
use monux_core::fsstorage::storage::NoteStorage;
use monux_core::index::{NoteIndex, NoteMeta, path_key, resolve_note_path};

pub struct CommandContext {
    storage: StorageContext,
}

impl CommandContext {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            storage: StorageContext::new()?,
        })
    }

    pub fn init(&self) -> anyhow::Result<()> {
        self.storage.init_app_dir()
    }

    pub fn config_path(&self) -> std::path::PathBuf {
        self.storage.config_path()
    }

    pub fn load_config(&self) -> anyhow::Result<Config> {
        self.storage.load_config()
    }

    pub fn open_note_index(&self) -> anyhow::Result<NoteIndex> {
        self.storage.open_note_index()
    }

    pub fn open_note_storage(&self) -> anyhow::Result<NoteStorage> {
        self.storage.open_note_storage()
    }
}

pub fn resolve_note_reference(index: &NoteIndex, raw: &str) -> anyhow::Result<NoteMeta> {
    let query = raw.trim();
    if query.is_empty() {
        anyhow::bail!("note reference is empty");
    }

    let rel = resolve_note_path(query);
    if !rel.as_os_str().is_empty()
        && let Some(note) = index.get_path(&rel)?
    {
        return Ok(note);
    }

    let notes = index.list()?;
    let q_lower = query.to_lowercase();

    let title_exact = notes
        .iter()
        .filter(|note| note.title.to_lowercase() == q_lower)
        .cloned()
        .collect::<Vec<_>>();
    if title_exact.len() == 1 {
        return Ok(title_exact[0].clone());
    }
    if title_exact.len() > 1 {
        anyhow::bail!(
            "multiple notes matched title '{}'; use note
path",
            query
        );
    }

    let contains = notes
        .iter()
        .filter(|note| {
            note.title.to_lowercase().contains(&q_lower)
                || path_key(&note.path).to_lowercase().contains(&q_lower)
        })
        .cloned()
        .collect::<Vec<_>>();
    if contains.len() == 1 {
        return Ok(contains[0].clone());
    }
    if contains.len() > 1 {
        anyhow::bail!(
            "multiple notes matched '{}'; refine query or
use path",
            query
        );
    }

    let fuzzy = index.find(query)?;
    if fuzzy.len() == 1 {
        return Ok(fuzzy[0].clone());
    }
    if fuzzy.is_empty() {
        anyhow::bail!("note '{}' not found", query);
    }

    anyhow::bail!("multiple notes matched '{}'; use note path", query)
}
