//! Where GameHub keeps its own data: the library, the settings and the
//! play activity. All of it is small JSON in the app's data directory.
//! Nothing here ever writes inside a launcher's own folders.

use std::path::PathBuf;

use gamehub_detect::model::Game;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MetadataKeys {
    pub igdb_client_id: String,
    pub igdb_client_secret: String,
    pub steam_grid_db_key: String,
}

impl Default for MetadataKeys {
    fn default() -> Self {
        Self {
            igdb_client_id: String::new(),
            igdb_client_secret: String::new(),
            steam_grid_db_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub enabled: bool,
    pub provider: String,
    pub api_key: String,
    pub model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            // Gemini's free tier is the default because the assistant is a
            // convenience, and nobody should need a paid key to try it.
            provider: "gemini".into(),
            api_key: String::new(),
            model: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CloudSettings {
    pub enabled: bool,
    pub api_url: String,
}

impl Default for CloudSettings {
    fn default() -> Self {
        Self { enabled: false, api_url: String::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub start_with_windows: bool,
    pub minimise_to_tray: bool,
    pub scan_interval_minutes: u64,
    pub auto_add_new_games: bool,
    /// Playtime counts only while the game is the window in front. Off (the
    /// default), the whole time the game is open counts — though a sleeping
    /// PC never does.
    pub track_active_only: bool,
    /// False until the user has touched `track_active_only` themselves, so a
    /// changed default can be applied once without overriding a real choice.
    pub active_only_chosen: bool,
    /// Minutes without keyboard or mouse before the clock pauses. 0 = never.
    pub idle_minutes: u64,
    /// Privacy: when false, no playtime, streak or calendar data is recorded.
    /// Launching games is unaffected.
    pub track_activity: bool,
    /// Minutes of play before a day counts towards a streak.
    pub streak_threshold_minutes: u64,
    /// Where screenshots are written. Empty means the default under app data.
    pub screenshot_folder: String,
    /// Which display the screenshot hotkey grabs.
    pub screenshot_monitor: usize,
    /// Clipboard history on or off. Off also stops the poller entirely.
    pub clipboard_enabled: bool,
    /// Accent colour override, e.g. "#6d7cff". Empty means the theme's own.
    pub accent: String,
    /// "comfortable" or "compact".
    pub density: String,
    /// Absolute path to a background image, or empty.
    pub background_image: String,
    /// Instant replay. Off by default: it costs CPU, so it is opt-in.
    pub replay: crate::replay::ReplaySettings,
    pub extra_game_folders: Vec<String>,
    pub metadata: MetadataKeys,
    pub ai: AiSettings,
    pub cloud: CloudSettings,
    /// Interface language code (`en`, `nb`, ...). English until chosen.
    pub language: String,
    /// False until the language screen has been answered once — so existing
    /// installs see it once after updating, and new ones on first start.
    pub language_chosen: bool,
    /// The little popup over the game when a shortcut does something while
    /// GameHub is hidden, and whether it plings.
    pub overlay_popup: bool,
    pub overlay_sound: bool,
    /// The short logo animation and chime when GameHub opens on screen.
    pub startup_animation: bool,
    pub startup_sound: bool,
    /// Also play it when the window comes back after being closed to the
    /// tray — closing with X is "closing GameHub" to most people.
    pub startup_on_reopen: bool,
    /// Random id made on first run, for the anonymous usage statistics.
    /// Only created once consent exists, and cleared again when it is
    /// withdrawn with "delete my statistics data".
    pub install_id: String,
    /// Terms accepted and consent given, with versions and timestamps.
    pub legal: crate::legal::LegalSettings,
    /// Desktop background and lock screen picture GameHub set for Windows.
    pub wallpapers: crate::wallpaper::WallpaperSettings,
    pub theme: String,
    pub onboarded: bool,
    /// The update version the user chose "Later" for. Cleared automatically by
    /// a newer one appearing, so nobody is nagged about the same release twice.
    pub dismissed_update_version: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            minimise_to_tray: true,
            // Watchers do the real work; the timer is a safety net for the
            // launchers that write their manifests somewhere unwatched.
            scan_interval_minutes: 15,
            auto_add_new_games: true,
            track_active_only: false,
            active_only_chosen: false,
            idle_minutes: 10,
            track_activity: true,
            streak_threshold_minutes: 15,
            screenshot_folder: String::new(),
            screenshot_monitor: 0,
            clipboard_enabled: true,
            accent: String::new(),
            density: "comfortable".into(),
            background_image: String::new(),
            replay: crate::replay::ReplaySettings::default(),
            extra_game_folders: Vec::new(),
            metadata: MetadataKeys::default(),
            ai: AiSettings::default(),
            cloud: CloudSettings::default(),
            language: "en".into(),
            language_chosen: false,
            overlay_popup: true,
            overlay_sound: true,
            startup_animation: true,
            startup_sound: true,
            startup_on_reopen: true,
            install_id: String::new(),
            legal: crate::legal::LegalSettings::default(),
            wallpapers: crate::wallpaper::WallpaperSettings::default(),
            theme: "nattbla".into(),
            onboarded: false,
            dismissed_update_version: None,
        }
    }
}

/// What happened to a file that could not be read. Surfaced in the app so a
/// recovery is something the user is told about, not something they discover
/// by noticing games are missing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryNote {
    pub file: String,
    /// Where the unreadable original was kept.
    pub kept_at: String,
    /// True when a backup was put back in its place.
    pub restored: bool,
    pub reason: String,
}

pub struct Paths {
    pub data_dir: PathBuf,
    /// Filled in during loading, read once by the UI. A Mutex because loading
    /// happens on whichever thread got there first.
    recoveries: std::sync::Mutex<Vec<RecoveryNote>>,
}

impl Paths {
    pub fn new(data_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(data_dir.join("artwork"));
        let _ = std::fs::create_dir_all(data_dir.join("backgrounds"));
        let _ = std::fs::create_dir_all(gamehub_detect::backup::backups_dir(&data_dir));
        Self { data_dir, recoveries: std::sync::Mutex::new(Vec::new()) }
    }

    pub fn note_recovery(&self, note: RecoveryNote) {
        if let Ok(mut notes) = self.recoveries.lock() {
            notes.push(note);
        }
    }

    pub fn recoveries(&self) -> Vec<RecoveryNote> {
        self.recoveries.lock().map(|n| n.clone()).unwrap_or_default()
    }

    /// Where a chosen background image is copied to.
    ///
    /// The user picks a file from anywhere on their disk. Pointing at it in
    /// place means it breaks the moment they move or delete it, and it sits
    /// outside anything the web view is allowed to read. Copying it here fixes
    /// both, and means the background survives a backup and restore too.
    pub fn backgrounds(&self) -> PathBuf {
        self.data_dir.join("backgrounds")
    }

    /// Where chosen wallpapers are copied to, for the same reasons.
    pub fn wallpapers(&self) -> PathBuf {
        self.data_dir.join("wallpapers")
    }

    /// Takes a snapshot if this is the first launch after an update.
    ///
    /// Called before anything is loaded, so the copy is of the previous
    /// version's data exactly as it left it.
    pub fn snapshot_if_updated(&self, app_version: &str) -> Option<gamehub_detect::backup::Snapshot> {
        use gamehub_detect::backup;

        let marker = backup::read_marker(&self.data_dir);
        let now = gamehub_detect::now_iso8601();
        let reason = backup::upgrade_reason(marker.as_ref(), app_version, backup::has_data(&self.data_dir));

        let snapshot = match &reason {
            Some(reason) => {
                let id = format!("{}-{}", backup::slug(&now), reason);
                match backup::take_snapshot(&self.data_dir, &id, reason, app_version, &now) {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        // A failed backup must not stop the app starting, but it
                        // must be loud in the log.
                        eprintln!("could not back up before updating: {error}");
                        None
                    }
                }
            }
            None => None,
        };

        let _ = backup::write_marker(&self.data_dir, app_version, &now);
        backup::prune(&self.data_dir, 10);
        snapshot
    }

    pub fn library(&self) -> PathBuf {
        self.data_dir.join("library.json")
    }

    pub fn settings(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    pub fn activity(&self) -> PathBuf {
        self.data_dir.join("activity.json")
    }

    /// Where fetched cover art is cached. Exposed to the web view through
    /// Tauri's asset protocol, scoped to this folder alone.
    pub fn artwork(&self) -> PathBuf {
        self.data_dir.join("artwork")
    }

    pub fn clips_index(&self) -> PathBuf {
        self.data_dir.join("clips.json")
    }

    /// Where finished clips go.
    pub fn clip_root(&self, configured: &str) -> PathBuf {
        if configured.trim().is_empty() {
            self.data_dir.join("Clips")
        } else {
            PathBuf::from(configured)
        }
    }

    /// The rolling buffer's scratch folder. Deliberately inside app data and
    /// never inside the user's clip folder — it is churned constantly.
    pub fn replay_scratch(&self) -> PathBuf {
        self.data_dir.join("replay-buffer")
    }

    pub fn clipboard(&self) -> PathBuf {
        self.data_dir.join("clipboard.json")
    }

    /// The freeze-point index. The points themselves — save copies and
    /// pictures — live beside the replay clips, under the game's folder.
    pub fn freezes_index(&self) -> PathBuf {
        self.data_dir.join("freezes.json")
    }

    pub fn screenshots_index(&self) -> PathBuf {
        self.data_dir.join("screenshots.json")
    }

    pub fn shortcuts(&self) -> PathBuf {
        self.data_dir.join("shortcuts.json")
    }

    /// Where screenshot files go: the folder the user chose, or ours.
    pub fn screenshot_root(&self, configured: &str) -> PathBuf {
        if configured.trim().is_empty() {
            self.data_dir.join("Screenshots")
        } else {
            PathBuf::from(configured)
        }
    }

}

pub fn load_library(paths: &Paths) -> Vec<Game> {
    read_json(paths, &paths.library())
}

/// Written to a temporary file and renamed, so a crash mid-write cannot leave
/// the user with an empty library on next start.
pub fn save_library(paths: &Paths, games: &[Game]) -> std::io::Result<()> {
    write_atomic(&paths.library(), &serde_json::to_vec_pretty(games)?)
}

pub fn load_settings(paths: &Paths) -> Settings {
    read_json(paths, &paths.settings())
}

pub fn save_settings(paths: &Paths, settings: &Settings) -> std::io::Result<()> {
    write_atomic(&paths.settings(), &serde_json::to_vec_pretty(settings)?)
}

pub fn load_activity(paths: &Paths) -> gamehub_detect::activity::Activity {
    read_json(paths, &paths.activity())
}

pub fn save_activity(
    paths: &Paths,
    activity: &gamehub_detect::activity::Activity,
) -> std::io::Result<()> {
    write_atomic(&paths.activity(), &serde_json::to_vec_pretty(activity)?)
}

pub fn load_clipboard(paths: &Paths) -> gamehub_detect::clipboard::ClipboardHistory {
    read_json(paths, &paths.clipboard())
}

pub fn save_clipboard(
    paths: &Paths,
    history: &gamehub_detect::clipboard::ClipboardHistory,
) -> std::io::Result<()> {
    write_atomic(&paths.clipboard(), &serde_json::to_vec_pretty(history)?)
}

pub fn load_screenshots(paths: &Paths) -> gamehub_detect::media::ScreenshotLibrary {
    read_json(paths, &paths.screenshots_index())
}

pub fn save_screenshots(
    paths: &Paths,
    library: &gamehub_detect::media::ScreenshotLibrary,
) -> std::io::Result<()> {
    write_atomic(&paths.screenshots_index(), &serde_json::to_vec_pretty(library)?)
}

pub fn load_clips(paths: &Paths) -> crate::replay::ClipLibrary {
    read_json(paths, &paths.clips_index())
}

pub fn save_clips(paths: &Paths, library: &crate::replay::ClipLibrary) -> std::io::Result<()> {
    write_atomic(&paths.clips_index(), &serde_json::to_vec_pretty(library)?)
}

pub fn load_freezes(paths: &Paths) -> gamehub_detect::freeze::FreezeLibrary {
    read_json(paths, &paths.freezes_index())
}

pub fn save_freezes(paths: &Paths, library: &gamehub_detect::freeze::FreezeLibrary) -> std::io::Result<()> {
    write_atomic(&paths.freezes_index(), &serde_json::to_vec_pretty(library)?)
}

pub fn load_bindings(paths: &Paths) -> gamehub_detect::shortcuts::Bindings {
    read_json(paths, &paths.shortcuts())
}

pub fn save_bindings(
    paths: &Paths,
    bindings: &gamehub_detect::shortcuts::Bindings,
) -> std::io::Result<()> {
    write_atomic(&paths.shortcuts(), &serde_json::to_vec_pretty(bindings)?)
}

/// Reads one of GameHub's JSON files.
///
/// The old version of this function fell back to `T::default()` whenever the
/// file would not parse. That looked harmless and was not: the empty default
/// was saved back over the top on the next write, so a single unreadable byte
/// silently erased a library somebody had spent an evening curating.
///
/// Now a file that will not parse is moved aside under a new name — nothing is
/// ever destroyed — and the newest snapshot that *does* parse is put back in
/// its place. Only when there is no snapshot at all does this fall back to the
/// default, and by then the original bytes are still on disk to go back to.
fn read_json<T: Default + serde::de::DeserializeOwned>(paths: &Paths, path: &std::path::Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else {
        // No file yet. A fresh install, which is not a problem.
        return T::default();
    };

    match serde_json::from_str::<T>(&text) {
        Ok(value) => value,
        Err(error) => recover(paths, path, &error.to_string()),
    }
}

fn recover<T: Default + serde::de::DeserializeOwned>(
    paths: &Paths,
    path: &std::path::Path,
    why: &str,
) -> T {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let stamp = gamehub_detect::backup::slug(&gamehub_detect::now_iso8601());

    eprintln!("{name} could not be read ({why}); it will be kept and not overwritten");

    let moved = gamehub_detect::backup::quarantine(path, &stamp);

    let recovered = gamehub_detect::backup::newest_readable(&paths.data_dir, &name)
        .and_then(|source| std::fs::read_to_string(source).ok())
        .and_then(|text| serde_json::from_str::<T>(&text).ok());

    // Recorded so the app can tell the user plainly rather than looking like it
    // quietly forgot everything.
    paths.note_recovery(RecoveryNote {
        file: name,
        kept_at: moved.map(|p| p.display().to_string()).unwrap_or_default(),
        restored: recovered.is_some(),
        reason: why.to_string(),
    });

    match recovered {
        Some(value) => {
            eprintln!("  restored from the newest backup");
            value
        }
        None => {
            eprintln!("  no backup held a readable copy; starting empty, original kept");
            T::default()
        }
    }
}

fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}
