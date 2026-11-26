//! Views kept by what they were made from: the file's path, mtime and size,
//! and the plugin's version (design §3).

use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use serde_json::Value;

use crate::{blocks::Block, manifest::Plugin};

/// How many views stay in memory.
pub(crate) const KEPT: usize = 32;

/// What a cached view was made from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Key {
    path: PathBuf,
    modified: Option<u128>,
    size: u64,
    plugin: String,
    version: String,
}

impl Key {
    /// The key of `path` as `plugin` sees it now.
    pub fn new(path: &Path, plugin: &Plugin) -> Option<Key> {
        let meta = std::fs::metadata(path).ok()?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|since| since.as_nanos());
        Some(Key {
            path: path.to_path_buf(),
            modified,
            size: meta.len(),
            plugin: plugin.manifest.id.clone(),
            version: plugin.manifest.version.clone(),
        })
    }
}

/// One view as it was made.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cached {
    pub blocks: Vec<Block>,
    pub meta: Value,
}

/// A few views, the most recently used first.
#[derive(Debug)]
pub(crate) struct Cache {
    entries: Vec<(Key, Cached)>,
    kept: usize,
}

impl Cache {
    pub fn new(kept: usize) -> Cache {
        Cache {
            entries: Vec::new(),
            kept,
        }
    }

    /// The view made from `key`, moved to the front.
    pub fn get(&mut self, key: &Key) -> Option<Cached> {
        let at = self.entries.iter().position(|(known, _)| known == key)?;
        let entry = self.entries.remove(at);
        let cached = entry.1.clone();
        self.entries.insert(0, entry);
        Some(cached)
    }

    /// Keeps `view` for `key`, dropping the least recently used.
    pub fn put(&mut self, key: Key, view: Cached) {
        self.entries.retain(|(known, _)| *known != key);
        self.entries.insert(0, (key, view));
        self.entries.truncate(self.kept);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;

    fn plugin(version: &str) -> Plugin {
        Plugin {
            dir: PathBuf::from("/plugins/csv"),
            manifest: Manifest {
                id: "csv".into(),
                name: None,
                version: version.into(),
                api_version: 1,
                extensions: vec![".csv".into()],
                command: vec!["python".into()],
                timeout_ms: 3_000,
                enabled: true,
                capabilities: vec!["blocks@1".into()],
            },
        }
    }

    fn view(text: &str) -> Cached {
        Cached {
            blocks: vec![Block::Paragraph(text.into())],
            meta: Value::Null,
        }
    }

    #[test]
    fn a_key_follows_the_file_and_the_plugin() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("a.csv");
        std::fs::write(&path, "a,b\n").expect("a file");
        let key = Key::new(&path, &plugin("1")).expect("a key");
        assert_eq!(Key::new(&path, &plugin("1")).expect("a key"), key);
        assert_ne!(Key::new(&path, &plugin("2")).expect("a key"), key);
        std::fs::write(&path, "a,b\nc,d\n").expect("a file");
        assert_ne!(Key::new(&path, &plugin("1")).expect("a key"), key);
    }

    #[test]
    fn a_hit_moves_to_the_front_and_the_last_goes() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut cache = Cache::new(2);
        let mut keys = Vec::new();
        for name in ["a.csv", "b.csv", "c.csv", "d.csv"] {
            let path = dir.path().join(name);
            std::fs::write(&path, name).expect("a file");
            keys.push(Key::new(&path, &plugin("1")).expect("a key"));
        }
        cache.put(keys[0].clone(), view("a"));
        cache.put(keys[1].clone(), view("b"));
        cache.put(keys[2].clone(), view("c"));
        assert!(cache.get(&keys[0]).is_none(), "the oldest went");
        // The hit keeps b when d would otherwise drop it.
        assert!(cache.get(&keys[1]).is_some());
        cache.put(keys[3].clone(), view("d"));
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&keys[1]).is_some(), "b stayed");
        assert!(cache.get(&keys[2]).is_none(), "c went");
        assert!(cache.get(&keys[3]).is_some(), "d is in");
    }
}
