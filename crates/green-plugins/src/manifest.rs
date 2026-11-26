//! `manifest.json`, and the folder of plugins it describes (design §3).
//!
//! A manifest that cannot be read is a problem for the log, never a failure:
//! the plugin is skipped and everything else still loads.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// The only contract this build speaks; another `apiVersion` is refused.
pub const API_VERSION: u32 = 1;

/// The one capability this build offers; a plugin must declare it.
pub const BLOCKS: &str = "blocks@1";

/// `timeoutMs` when a manifest leaves it out.
pub const DEFAULT_TIMEOUT_MS: u64 = 3_000;

/// The timeouts a manifest may ask for.
const TIMEOUTS: std::ops::RangeInclusive<u64> = 50..=600_000;

/// What a plugin's folder declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// Its folder name's twin, and the cache key's plugin half.
    pub id: String,
    /// The name a menu could show; optional.
    pub name: Option<String>,
    /// Its own version; the cache key includes it.
    pub version: String,
    /// The contract it speaks.
    pub api_version: u32,
    /// Extensions it reads, each with a leading dot, lowercased.
    pub extensions: Vec<String>,
    /// The process to run, its working folder the plugin's.
    pub command: Vec<String>,
    /// How long one run may take before it is killed.
    pub timeout_ms: u64,
    /// A plugin switched off still loads, and never runs.
    pub enabled: bool,
    /// What it can produce; `blocks@1` is the only one read.
    pub capabilities: Vec<String>,
}

/// As written; every field a later contract adds must be optional here.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Raw {
    id: String,
    name: Option<String>,
    version: String,
    api_version: u32,
    extensions: Vec<String>,
    command: Vec<String>,
    timeout_ms: Option<u64>,
    enabled: Option<bool>,
    capabilities: Option<Vec<String>>,
}

impl Manifest {
    /// `bytes` as a manifest, or why it is not one.
    pub fn parse(bytes: &[u8]) -> Result<Manifest, String> {
        // Notepad saves UTF-8 with a byte order mark; JSON itself has none.
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        let raw: Raw =
            serde_json::from_slice(bytes).map_err(|error| format!("invalid JSON: {error}"))?;
        Self::check(raw)
    }

    fn check(raw: Raw) -> Result<Manifest, String> {
        if raw.api_version != API_VERSION {
            return Err(format!(
                "apiVersion {} is not supported; this build speaks {API_VERSION}",
                raw.api_version
            ));
        }
        let id = raw.id.trim().to_string();
        if id.is_empty() || id.contains(['/', '\\']) {
            return Err("id must be a name, not a path".into());
        }
        let version = raw.version.trim().to_string();
        if version.is_empty() {
            return Err("version is empty".into());
        }
        let mut extensions = Vec::new();
        for extension in &raw.extensions {
            let extension = normalize_extension(extension)
                .ok_or_else(|| format!("extension {extension:?} is not a file extension"))?;
            if !extensions.contains(&extension) {
                extensions.push(extension);
            }
        }
        if extensions.is_empty() {
            return Err("extensions is empty".into());
        }
        if raw.command.is_empty() || raw.command[0].trim().is_empty() {
            return Err("command is empty".into());
        }
        let timeout_ms = raw.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
        if !TIMEOUTS.contains(&timeout_ms) {
            return Err(format!(
                "timeoutMs {timeout_ms} is outside {}..={}",
                TIMEOUTS.start(),
                TIMEOUTS.end()
            ));
        }
        let capabilities = match raw.capabilities {
            Some(list) => list
                .iter()
                .map(|name| name.trim().to_ascii_lowercase())
                .filter(|name| !name.is_empty())
                .collect(),
            // A manifest from before capabilities names only the one contract.
            None => vec![BLOCKS.to_string()],
        };
        Ok(Manifest {
            id,
            name: raw.name,
            version,
            api_version: raw.api_version,
            extensions,
            command: raw.command,
            timeout_ms,
            enabled: raw.enabled.unwrap_or(true),
            capabilities,
        })
    }

    /// Whether it produces the one capability this build reads.
    pub fn speaks_blocks(&self) -> bool {
        self.capabilities.iter().any(|name| name == BLOCKS)
    }
}

/// `extension` as this build matches it: lowercase, one leading dot.
pub fn normalize_extension(extension: &str) -> Option<String> {
    let extension = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if extension.is_empty() || extension.contains(['/', '\\', '.']) {
        return None;
    }
    Some(format!(".{extension}"))
}

/// A plugin folder that loaded.
#[derive(Clone, Debug)]
pub struct Plugin {
    /// Its folder, and the working folder of its process.
    pub dir: PathBuf,
    pub manifest: Manifest,
}

/// A folder that did not load, and why; it is logged, never fatal.
#[derive(Clone, Debug)]
pub struct Problem {
    pub dir: PathBuf,
    pub why: String,
}

/// Every plugin folder found, and every one skipped.
#[derive(Clone, Debug, Default)]
pub struct Registry {
    plugins: Vec<Plugin>,
    problems: Vec<Problem>,
}

impl Registry {
    /// Reads every `<dir>/*/manifest.json`; a missing `dir` is no plugins.
    pub fn load(dir: &Path) -> Registry {
        let mut registry = Registry::default();
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return registry,
            Err(error) => {
                registry.problems.push(Problem {
                    dir: dir.to_path_buf(),
                    why: format!("the folder did not list: {error}"),
                });
                return registry;
            }
        };
        let mut folders: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        folders.sort();
        for folder in folders {
            match Self::one(&folder) {
                Ok(plugin) => {
                    if let Some(first) = registry
                        .plugins
                        .iter()
                        .find(|known| known.manifest.id == plugin.manifest.id)
                    {
                        registry.problems.push(Problem {
                            dir: folder,
                            why: format!(
                                "id {:?} is already used by {}",
                                plugin.manifest.id,
                                first.dir.display()
                            ),
                        });
                        continue;
                    }
                    for extension in &plugin.manifest.extensions {
                        if let Some(owner) = registry
                            .plugins
                            .iter()
                            .find(|known| known.manifest.extensions.contains(extension))
                        {
                            log::info!(
                                "plugins: {extension} stays {}/{}; {} also declares it",
                                owner.manifest.id,
                                owner.manifest.version,
                                folder.display()
                            );
                        }
                    }
                    registry.plugins.push(plugin);
                }
                Err(why) => registry.problems.push(Problem { dir: folder, why }),
            }
        }
        // The first by id answers an extension, whatever the folders' names.
        registry
            .plugins
            .sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
        registry
    }

    fn one(folder: &Path) -> Result<Plugin, String> {
        let path = folder.join("manifest.json");
        let bytes = fs::read(&path).map_err(|error| format!("manifest.json: {error}"))?;
        let manifest = Manifest::parse(&bytes)?;
        Ok(Plugin {
            dir: folder.to_path_buf(),
            manifest,
        })
    }

    /// Every plugin that loaded, enabled or not.
    pub fn plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    /// Every folder skipped, and why.
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    /// The first plugin, by id, that names `extension`; enabled or not.
    pub fn find(&self, extension: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|plugin| {
            plugin
                .manifest
                .extensions
                .iter()
                .any(|own| own == extension)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
        "id": "csv", "name": "CSV", "version": "0.1.0", "apiVersion": 1,
        "extensions": [".csv"], "command": ["python", "csv_plugin.py"],
        "timeoutMs": 3000, "enabled": true, "capabilities": ["blocks@1"]
    }"#;

    fn parsed(json: &str) -> Result<Manifest, String> {
        Manifest::parse(json.as_bytes())
    }

    #[test]
    fn the_design_example_loads() {
        let manifest = parsed(GOOD).expect("a manifest");
        assert_eq!(manifest.id, "csv");
        assert_eq!(manifest.extensions, [".csv"]);
        assert_eq!(manifest.timeout_ms, 3_000);
        assert!(manifest.enabled);
        assert!(manifest.speaks_blocks());
    }

    #[test]
    fn a_field_from_a_later_contract_is_ignored() {
        let json = GOOD.replace("\"id\":", "\"icon\": \"table\", \"id\":");
        assert_eq!(parsed(&json).expect("a manifest").id, "csv");
    }

    #[test]
    fn what_a_manifest_may_leave_out() {
        let json = r#"{
            "id": "log", "version": "1", "apiVersion": 1,
            "extensions": ["log"], "command": ["python", "log.py"]
        }"#;
        let manifest = parsed(json).expect("a manifest");
        assert_eq!(manifest.extensions, [".log"]);
        assert_eq!(manifest.timeout_ms, DEFAULT_TIMEOUT_MS);
        assert!(manifest.enabled);
        assert_eq!(manifest.capabilities, [BLOCKS]);
    }

    #[test]
    fn a_byte_order_mark_is_not_a_syntax_error() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(GOOD.as_bytes());
        assert_eq!(Manifest::parse(&bytes).expect("a manifest").id, "csv");
    }

    #[test]
    fn a_broken_manifest_is_refused_with_a_reason() {
        assert!(parsed("{").expect_err("syntax").contains("invalid JSON"));
        let other = GOOD.replace("\"apiVersion\": 1", "\"apiVersion\": 2");
        assert!(
            parsed(&other)
                .expect_err("a version")
                .contains("apiVersion 2")
        );
        let word = GOOD.replace("\"apiVersion\": 1", "\"apiVersion\": \"1\"");
        assert!(parsed(&word).expect_err("a type").contains("invalid"));
        let none = GOOD.replace("\"command\": [\"python\", \"csv_plugin.py\"],", "");
        assert!(parsed(&none).expect_err("no command").contains("command"));
        let empty = GOOD.replace("[\"python\", \"csv_plugin.py\"]", "[]");
        assert!(parsed(&empty).expect_err("no command").contains("command"));
        let quick = GOOD.replace("3000", "0");
        assert!(
            parsed(&quick)
                .expect_err("a timeout")
                .contains("timeoutMs 0")
        );
    }

    #[test]
    fn a_folder_is_one_plugin_and_a_bad_one_is_a_problem() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let good = dir.path().join("csv");
        let bad = dir.path().join("broken");
        fs::create_dir_all(&good).expect("a folder");
        fs::create_dir_all(&bad).expect("a folder");
        fs::write(good.join("manifest.json"), GOOD).expect("a manifest");
        fs::write(bad.join("manifest.json"), "{\"apiVersion\": 9}").expect("a manifest");
        let registry = Registry::load(dir.path());
        assert_eq!(registry.plugins().len(), 1);
        assert_eq!(registry.plugins()[0].manifest.id, "csv");
        assert_eq!(registry.problems().len(), 1);
        assert!(registry.find(".csv").is_some());
        assert!(registry.find(".CSV").is_none(), "extensions are normalized");
    }

    #[test]
    fn a_duplicate_id_keeps_the_first_folder() {
        let dir = tempfile::tempdir().expect("a temp dir");
        for folder in ["a", "b"] {
            let path = dir.path().join(folder);
            fs::create_dir_all(&path).expect("a folder");
            fs::write(path.join("manifest.json"), GOOD).expect("a manifest");
        }
        let registry = Registry::load(dir.path());
        assert_eq!(registry.plugins().len(), 1);
        assert_eq!(registry.problems().len(), 1);
        assert!(registry.problems()[0].why.contains("already used"));
    }

    #[test]
    fn no_folder_is_no_plugins_and_no_problem() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let registry = Registry::load(&dir.path().join("missing"));
        assert!(registry.plugins().is_empty());
        assert!(registry.problems().is_empty());
    }
}
