use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};


use notify::{Config, Event, RecommendedWatcher, RecursiveMode,
Watcher};


pub struct VaultWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<PathBuf>,
}


impl VaultWatcher {
    pub fn start(notes_root: &Path) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel::<PathBuf>();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                if let Ok(event) = result {
                    for path in event.paths {
                        let _ = tx.send(path);
                    }
                }
            },
            Config::default(),
        )?;


        watcher.watch(notes_root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }


    pub fn drain(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        while let Ok(path) = self.rx.try_recv() {
            out.push(path);
        }
        out
    }
}


