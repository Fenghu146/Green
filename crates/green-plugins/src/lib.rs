//! Green's external-process plugins (design §3).
//!
//! A plugin is a folder under `%APPDATA%\Green\plugins\<id>\` holding a
//! `manifest.json` and whatever it runs. Each open starts it once, writes one
//! JSON request to its stdin and reads one JSON answer from its stdout. A bad
//! manifest, a missing program, a crash or a timeout is a reason for a plain
//! text view, never a reason the app fails.

mod blocks;
mod cache;
mod manifest;
mod supervisor;

use std::{
    fmt, fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use base64::Engine as _;
use serde_json::Value;

pub use blocks::{AlertLevel, Align, Block, to_markdown};
pub use manifest::{API_VERSION, BLOCKS, Manifest, Plugin, Problem};

use cache::{Cache, Cached, Key};
use supervisor::Request;

/// A file larger than this is not sent; the plugin reads its path itself.
pub const CONTENT_LIMIT: u64 = 8 * 1024 * 1024;

/// The plugin folders, and the views they made lately.
pub struct Host {
    dir: PathBuf,
    registry: Mutex<manifest::Registry>,
    cache: Mutex<Cache>,
}

impl Host {
    /// Reads `dir`'s plugins; a missing folder is no plugins, not an error.
    pub fn load(dir: impl Into<PathBuf>) -> Host {
        let dir = dir.into();
        let registry = manifest::Registry::load(&dir);
        Host {
            dir,
            registry: Mutex::new(registry),
            cache: Mutex::new(Cache::new(cache::KEPT)),
        }
    }

    /// The folder read.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every plugin that loaded, switched on or not.
    pub fn plugins(&self) -> Vec<Plugin> {
        self.registry().plugins().to_vec()
    }

    /// Every folder skipped, and why.
    pub fn problems(&self) -> Vec<Problem> {
        self.registry().problems().to_vec()
    }

    /// Whether an enabled plugin with this build's capability reads `path`.
    pub fn serves(&self, path: &Path) -> bool {
        self.registry()
            .find(&extension_of(path))
            .is_some_and(|plugin| plugin.manifest.enabled && plugin.manifest.speaks_blocks())
    }

    /// The plugin that reads `extension`, read again if its files changed.
    fn ready(&self, extension: &str) -> Result<Plugin, Reason> {
        let mut registry = self.registry();
        let Some(found) = registry.find(extension).cloned() else {
            return Err(Reason::NoPlugin(extension.to_string()));
        };
        let plugin = if found.fingerprint.current() {
            found
        } else {
            // A changed script or manifest is picked up here, not at launch.
            let began = std::time::Instant::now();
            let reloaded = registry.reload(&found.dir);
            self.cache().forget(&found.manifest.id);
            match &reloaded {
                Ok(()) => log::info!(
                    "plugins: {} changed; reloaded in {} µs",
                    found.manifest.id,
                    began.elapsed().as_micros()
                ),
                Err(why) => log::warn!(
                    "plugins: {} changed and did not load: {why}",
                    found.manifest.id
                ),
            }
            reloaded.map_err(Reason::Manifest)?;
            let Some(plugin) = registry.find(extension).cloned() else {
                return Err(Reason::NoPlugin(extension.to_string()));
            };
            self.cache().forget(&plugin.manifest.id);
            plugin
        };
        if !plugin.manifest.enabled {
            return Err(Reason::Disabled(plugin.manifest.id.clone()));
        }
        if !plugin.manifest.speaks_blocks() {
            return Err(Reason::NoCapability(plugin.manifest.id.clone()));
        }
        Ok(plugin)
    }

    /// `path`'s blocks, or why they are not here.
    pub fn open(&self, path: &Path) -> View {
        let extension = extension_of(path);
        let plugin = match self.ready(&extension) {
            Ok(plugin) => plugin,
            Err(reason) => return View::Plain { reason },
        };
        let real = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let key = Key::new(&real, &plugin);
        if let Some(key) = &key
            && let Some(cached) = self.cache().get(key)
        {
            return View::Blocks {
                plugin: plugin.manifest.id.clone(),
                blocks: cached.blocks,
                meta: cached.meta,
                cached: true,
            };
        }
        let size = match fs::metadata(&real) {
            Ok(meta) => meta.len(),
            Err(error) => {
                return View::Plain {
                    reason: Reason::Read(error.to_string()),
                };
            }
        };
        let content = if size <= CONTENT_LIMIT {
            match fs::read(&real) {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    return View::Plain {
                        reason: Reason::Read(error.to_string()),
                    };
                }
            }
        } else {
            None
        };
        let omitted = content.is_none();
        let shown = real.to_string_lossy();
        let request = Request {
            api_version: API_VERSION,
            path: &shown,
            ext: &extension,
            content: content.map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes)),
            content_omitted: omitted,
            capabilities: &[BLOCKS],
        };
        match supervisor::run(&plugin, &request) {
            Ok(response) => {
                if let Some(key) = key {
                    self.cache().put(
                        key,
                        Cached {
                            blocks: response.blocks.clone(),
                            meta: response.meta.clone(),
                        },
                    );
                }
                View::Blocks {
                    plugin: plugin.manifest.id.clone(),
                    blocks: response.blocks,
                    meta: response.meta,
                    cached: false,
                }
            }
            Err(failure) => View::Plain {
                reason: failure.into(),
            },
        }
    }

    fn registry(&self) -> std::sync::MutexGuard<'_, manifest::Registry> {
        self.registry
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn cache(&self) -> std::sync::MutexGuard<'_, Cache> {
        self.cache
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

/// What one open produced.
#[derive(Clone, Debug, PartialEq)]
pub enum View {
    /// The plugin's blocks.
    Blocks {
        /// The plugin that made them.
        plugin: String,
        blocks: Vec<Block>,
        /// Whatever it put beside them.
        meta: Value,
        /// Whether this open read the cache instead of running it.
        cached: bool,
    },
    /// No blocks: the file shows as text, and this says why.
    Plain { reason: Reason },
}

impl View {
    /// The blocks, when the plugin produced them.
    pub fn blocks(&self) -> Option<&[Block]> {
        match self {
            View::Blocks { blocks, .. } => Some(blocks),
            View::Plain { .. } => None,
        }
    }

    /// Why there are no blocks, when there are none.
    pub fn reason(&self) -> Option<&Reason> {
        match self {
            View::Blocks { .. } => None,
            View::Plain { reason } => Some(reason),
        }
    }

    /// The blocks as one Markdown text; `None` for a plain view.
    pub fn markdown(&self) -> Option<String> {
        self.blocks().map(to_markdown)
    }
}

/// Why a file has no plugin view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// No plugin names the file's extension.
    NoPlugin(String),
    /// The plugin that names it is switched off.
    Disabled(String),
    /// The plugin does not produce blocks.
    NoCapability(String),
    /// The plugin's manifest broke after it loaded.
    Manifest(String),
    /// The file could not be read.
    Read(String),
    /// The plugin's process did not start.
    Spawn(String),
    /// The request did not serialize.
    Request(String),
    /// The host could not wait for the process.
    Wait(String),
    /// The plugin was killed at its `timeoutMs`.
    Timeout(u64),
    /// It ended with a failure code.
    Exit { code: Option<i32>, stderr: String },
    /// It wrote no JSON this build can read.
    InvalidJson(String),
    /// It answered in another contract.
    Api { found: u32 },
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::NoPlugin(extension) if extension.is_empty() => {
                write!(f, "no plugin reads files without an extension")
            }
            Reason::NoPlugin(extension) => write!(f, "no plugin reads {extension} files"),
            Reason::Disabled(id) => write!(f, "plugin {id} is switched off"),
            Reason::NoCapability(id) => write!(f, "plugin {id} produces no blocks"),
            Reason::Manifest(why) => write!(f, "the plugin's manifest is broken: {why}"),
            Reason::Read(why) => write!(f, "the file did not read: {why}"),
            Reason::Spawn(why) => write!(f, "the plugin did not start: {why}"),
            Reason::Request(why) => write!(f, "the request did not serialize: {why}"),
            Reason::Wait(why) => write!(f, "the plugin could not be waited for: {why}"),
            Reason::Timeout(ms) => {
                write!(f, "the plugin did not finish within {ms} ms and was killed")
            }
            Reason::Exit { code, stderr } if stderr.is_empty() => match code {
                Some(code) => write!(f, "the plugin ended with exit code {code}"),
                None => write!(f, "the plugin ended without an exit code"),
            },
            Reason::Exit { code, stderr } => match code {
                Some(code) => write!(f, "the plugin ended with exit code {code}: {stderr}"),
                None => write!(f, "the plugin ended without an exit code: {stderr}"),
            },
            Reason::InvalidJson(why) => write!(f, "the plugin's answer is not JSON: {why}"),
            Reason::Api { found } => {
                write!(f, "the plugin's apiVersion {found} is not {API_VERSION}")
            }
        }
    }
}

impl From<supervisor::Failure> for Reason {
    fn from(failure: supervisor::Failure) -> Reason {
        match failure {
            supervisor::Failure::Spawn(why) => Reason::Spawn(why),
            supervisor::Failure::Request(why) => Reason::Request(why),
            supervisor::Failure::Wait(why) => Reason::Wait(why),
            supervisor::Failure::Timeout(ms) => Reason::Timeout(ms),
            supervisor::Failure::Exit { code, stderr } => Reason::Exit { code, stderr },
            supervisor::Failure::InvalidJson(why) => Reason::InvalidJson(why),
            supervisor::Failure::Api { found } => Reason::Api { found },
        }
    }
}

/// `path`'s extension as plugins name it: lowercase, one leading dot.
pub fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{}", extension.to_ascii_lowercase()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_extension_reads_the_one_way() {
        assert_eq!(extension_of(Path::new("a/b/Data.CSV")), ".csv");
        assert_eq!(extension_of(Path::new("a/b/data.tar.gz")), ".gz");
        assert_eq!(extension_of(Path::new("a/b/README")), "");
    }

    #[test]
    fn a_reason_says_what_happened() {
        assert_eq!(
            Reason::NoPlugin(".csv".into()).to_string(),
            "no plugin reads .csv files"
        );
        assert_eq!(
            Reason::Timeout(300).to_string(),
            "the plugin did not finish within 300 ms and was killed"
        );
        assert_eq!(
            Reason::Exit {
                code: Some(2),
                stderr: "bad".into()
            }
            .to_string(),
            "the plugin ended with exit code 2: bad"
        );
    }

    #[test]
    fn a_folder_with_no_plugins_serves_nothing() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let host = Host::load(dir.path().join("missing"));
        assert!(host.plugins().is_empty());
        assert!(!host.serves(Path::new("a.csv")));
        assert_eq!(
            host.open(Path::new("a.csv")),
            View::Plain {
                reason: Reason::NoPlugin(".csv".into())
            }
        );
    }

    #[test]
    fn a_switched_off_plugin_serves_nothing() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let folder = dir.path().join("csv");
        fs::create_dir_all(&folder).expect("a folder");
        fs::write(
            folder.join("manifest.json"),
            r#"{"id": "csv", "version": "1", "apiVersion": 1, "extensions": [".csv"],
                "command": ["python", "x.py"], "enabled": false}"#,
        )
        .expect("a manifest");
        let host = Host::load(dir.path());
        assert!(!host.serves(Path::new("a.csv")));
        assert_eq!(
            host.open(Path::new("a.csv")),
            View::Plain {
                reason: Reason::Disabled("csv".into())
            }
        );
    }

    #[test]
    fn a_plugin_without_blocks_serves_nothing() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let folder = dir.path().join("csv");
        fs::create_dir_all(&folder).expect("a folder");
        fs::write(
            folder.join("manifest.json"),
            r#"{"id": "csv", "version": "1", "apiVersion": 1, "extensions": [".csv"],
                "command": ["python", "x.py"], "capabilities": ["sheets@1"]}"#,
        )
        .expect("a manifest");
        let host = Host::load(dir.path());
        assert!(!host.serves(Path::new("a.csv")));
        assert_eq!(
            host.open(Path::new("a.csv")),
            View::Plain {
                reason: Reason::NoCapability("csv".into())
            }
        );
    }
}
