//! Folds kept per file: the text's headings and the outline's (design §5.3).
//!
//! The state lives in `%LOCALAPPDATA%\Green\view-state.json`, keyed by the
//! file's path. It never touches the `.md` file and makes no sidecar.

use std::{collections::BTreeMap, io, path::Path};

use gpui::{Context, Task};
use serde::{Deserialize, Serialize};

use crate::desk::Desk;

/// The file, in the app's data folder.
const FILE: &str = "view-state.json";

/// A save waits this long for the folds to rest.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(400);

/// A file's folds: the text's headings, and the outline's own.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FileView {
    /// Heading keys folded in the text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folded: Vec<String>,
    /// Heading keys folded in the outline; the two never follow each other.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panel: Vec<String>,
}

/// Every file's view state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewState {
    #[serde(default)]
    pub files: BTreeMap<String, FileView>,
}

/// The state, and its file.
pub struct Views {
    file: std::path::PathBuf,
    state: ViewState,
    /// Changes asked for, and the last one written.
    moves: u64,
    saved: u64,
    /// The write in flight, so a quit does not race it.
    _saving: Option<Task<()>>,
}

/// `path`'s key: its real path, so one file opened two ways is one entry.
pub fn key(path: &Path) -> String {
    dunce::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// Reads the state beside `data`; a bad file is reported, not overwritten.
pub fn load(data: &Path) -> io::Result<ViewState> {
    let file = data.join(FILE);
    match std::fs::read(&file) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ViewState::default()),
        Err(error) => Err(error),
    }
}

impl Views {
    /// The state in `data`; an unreadable one is reported and starts empty.
    pub fn start(data: &Path) -> (Views, Option<String>) {
        let (state, unread) = match load(data) {
            Ok(state) => (state, None),
            Err(error) => (ViewState::default(), Some(error.to_string())),
        };
        let views = Views {
            file: data.join(FILE),
            state,
            moves: 0,
            saved: 0,
            _saving: None,
        };
        (views, unread)
    }

    /// The headings folded in `path`'s text.
    pub fn folded(&self, path: &Path) -> Vec<String> {
        self.state
            .files
            .get(&key(path))
            .map(|view| view.folded.clone())
            .unwrap_or_default()
    }

    /// The headings folded in `path`'s outline.
    pub fn panel(&self, path: &Path) -> Vec<String> {
        self.state
            .files
            .get(&key(path))
            .map(|view| view.panel.clone())
            .unwrap_or_default()
    }

    /// Takes `path`'s folded headings; true when they changed.
    pub fn set_folded(&mut self, path: &Path, keys: Vec<String>) -> bool {
        self.set(path, Some(keys), None)
    }

    /// Takes `path`'s folded outline headings; true when they changed.
    pub fn set_panel(&mut self, path: &Path, keys: Vec<String>) -> bool {
        self.set(path, None, Some(keys))
    }

    fn set(
        &mut self,
        path: &Path,
        folded: Option<Vec<String>>,
        panel: Option<Vec<String>>,
    ) -> bool {
        let key = key(path);
        let mut changed = false;
        let entry = self.state.files.entry(key.clone()).or_default();
        for (slot, keys) in [(&mut entry.folded, folded), (&mut entry.panel, panel)] {
            let Some(mut keys) = keys else { continue };
            keys.retain(|key| !key.is_empty());
            keys.sort();
            keys.dedup();
            if *slot != keys {
                *slot = keys;
                changed = true;
            }
        }
        if self
            .state
            .files
            .get(&key)
            .is_some_and(|view| view.folded.is_empty() && view.panel.is_empty())
        {
            self.state.files.remove(&key);
        }
        changed
    }

    /// Writes the state once the folds rest; a later change takes over.
    pub fn settle(&mut self, cx: &mut Context<Desk>) {
        self.moves += 1;
        let (moves, file) = (self.moves, self.file.clone());
        self._saving = Some(cx.spawn(async move |desk, cx| {
            cx.background_executor().timer(SETTLE).await;
            let pending = desk.update(cx, |desk, _| {
                (desk.views.moves == moves && desk.views.saved != moves).then(|| desk.views.state.clone())
            });
            let Ok(Some(state)) = pending else {
                return;
            };
            let written = cx
                .background_executor()
                .spawn(async move {
                    let json = serde_json::to_vec_pretty(&state).map_err(io::Error::other)?;
                    ely_files::save_atomic(&file, &json, 0).map(|_| ())
                })
                .await;
            desk.update(cx, |desk, cx| desk.views_written(moves, written, cx))
                .ok();
        }));
    }

    /// Records a write that landed.
    pub(crate) fn written(&mut self, moves: u64) {
        self.saved = moves;
    }

    /// Writes a pending state now; quitting cannot wait.
    pub fn flush(&mut self) -> io::Result<()> {
        if self.moves == self.saved {
            return Ok(());
        }
        let json = serde_json::to_vec_pretty(&self.state).map_err(io::Error::other)?;
        ely_files::save_atomic(&self.file, &json, 0)?;
        self.saved = self.moves;
        Ok(())
    }
}
