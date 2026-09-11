//! What the web view is allowed to load from disk.
//!
//! Every image and video in GameHub is a real file: covers under `artwork`,
//! screenshots, replay clips, the background. The web view reaches them through
//! Tauri's asset protocol, which refuses anything outside an allowed folder.
//!
//! This module exists because the allowed list was previously written by hand in
//! `tauri.conf.json` as `$APPDATA/GameHub/artwork/**`, and `$APPDATA` already
//! *means* the app data folder — so the pattern resolved to
//! `%APPDATA%\com.gamehub.desktop\GameHub\artwork`, a folder that has never
//! existed. Nothing was ever in scope. That is why covers stayed blank, why the
//! background never appeared, and why screenshots could only be opened in
//! Explorer instead of viewed in the app.
//!
//! Granting it at runtime from the paths the app actually resolved removes the
//! chance of that class of mistake: there is no second copy of the path to get
//! wrong. It also covers the folders the user chooses themselves, which a static
//! pattern cannot know about.

use std::path::Path;

use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Grants the web view read access to everything GameHub needs to display.
///
/// Safe to call repeatedly — it is called again whenever the user changes a
/// folder in Settings, so a new screenshot folder works immediately rather than
/// after a restart.
pub fn grant(app: &AppHandle, state: &AppState) {
    let scope = app.asset_protocol_scope();
    let paths = &state.paths;

    // The app's own data folder: artwork, backgrounds, and the default
    // Screenshots and Clips folders all live under here.
    allow(&scope, &paths.data_dir, "app data");

    // Wherever the user has pointed screenshots and clips, which may be
    // anywhere on their disk.
    let (screenshot_folder, clip_folder, background) = {
        let inner = state.inner.lock();
        (
            inner.settings.screenshot_folder.clone(),
            inner.settings.replay.folder.clone(),
            inner.settings.background_image.clone(),
        )
    };

    for (configured, label) in [
        (screenshot_folder, "screenshot folder"),
        (clip_folder, "clip folder"),
    ] {
        if !configured.trim().is_empty() {
            allow(&scope, Path::new(&configured), label);
        }
    }

    // A background chosen before this version was stored as an absolute path
    // pointing anywhere. Newer ones are copied into app data and are already
    // covered above, but the old ones still need to work.
    if !background.trim().is_empty() {
        if let Some(parent) = Path::new(&background).parent() {
            allow(&scope, parent, "background image");
        }
    }
}

fn allow(scope: &tauri::scope::fs::Scope, path: &Path, label: &str) {
    if let Err(error) = scope.allow_directory(path, true) {
        // Not fatal: the app still launches and plays games, it just cannot
        // draw that particular set of images.
        eprintln!("could not grant access to the {label} at {}: {error}", path.display());
    }
}
