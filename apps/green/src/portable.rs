//! A `data` folder beside the executable makes this copy portable (S6).
//!
//! This is a product decision — how *this* program chooses its data folder —
//! not a platform capability, so it stays in the app layer: `crates/` must not
//! learn the word "portable". Move it into `ely-platform` only when a second
//! app needs the same policy.
//!
//! The data folder's own crate (`ely-platform`) is outside this line's
//! boundary, so the choice is made here, before any thread starts: it is the
//! same absolute root `ELY_DATA_ROOT` already names, and the folder is
//! `<exe>/data`, so the app writes `data/green/...` beside itself.

use std::env;

/// Adopts `<exe>/data` when it exists; an explicit `ELY_DATA_ROOT` wins.
pub(crate) fn adopt() {
    if env::var_os("ELY_DATA_ROOT").is_some() {
        return;
    }
    let Ok(exe) = env::current_exe() else {
        return;
    };
    let Some(data) = exe.parent().map(|folder| folder.join("data")) else {
        return;
    };
    if !data.is_dir() {
        return;
    }
    // SAFETY: this runs first in `main`, before any other thread exists.
    unsafe {
        env::set_var("ELY_DATA_ROOT", &data);
    }
    log::info!("run: portable data folder {}", data.display());
}
