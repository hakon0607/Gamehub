//! The only surface the front end can reach.
//!
//! Every command takes ids and simple values — never a path to execute, never a
//! command line. Anything that touches the filesystem resolves the path itself
//! from state it already trusts.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gamehub_adapters::scanner::{self, ScanOptions};
use gamehub_detect::activity::{self, DaySummary, Session, StreakSummary};
use gamehub_detect::model::{Game, ScanResult};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::AppState;
use crate::store::Settings;

type Reply<T> = Result<T, String>;

/* -------------------------------------------------------------------------- */
/* Library                                                                    */
/* -------------------------------------------------------------------------- */

#[tauri::command]
pub fn get_library(state: State<'_, Arc<AppState>>) -> Vec<Game> {
    state.inner.lock().library.clone()
}

#[tauri::command]
pub fn get_running_games(state: State<'_, Arc<AppState>>) -> Vec<String> {
    state.inner.lock().running.clone()
}

#[tauri::command]
pub async fn scan_now(app: AppHandle, state: State<'_, Arc<AppState>>) -> Reply<ScanResult> {
    let state = state.inner().clone();
    blocking(move || Ok(run_scan(&app, &state))).await
}

/// The one scan implementation. The watcher, the timer, the onboarding screen
/// and the Rescan button all end up here.
pub fn run_scan(app: &AppHandle, state: &Arc<AppState>) -> ScanResult {
    {
        let mut inner = state.inner.lock();
        if inner.scanning {
            return ScanResult {
                launchers: Vec::new(),
                games: inner.library.clone(),
                new_game_ids: Vec::new(),
                scanned_at: gamehub_detect::now_iso8601(),
                duration_ms: 0,
            };
        }
        inner.scanning = true;
    }

    let options = ScanOptions { extra_folders: state.extra_folders() };
    let result = {
        let mut inner = state.inner.lock();
        let mut library = std::mem::take(&mut inner.library);
        drop(inner);
        let result = scanner::scan_into(&mut library, &state.env, &options);
        let mut inner = state.inner.lock();
        inner.library = library;
        inner.scanning = false;
        result
    };

    state.persist_library();
    let _ = app.emit("library-updated", &result);

    // Artwork is fetched after the scan rather than during it, so a slow or
    // absent network can never hold up the library appearing.
    spawn_artwork_fetch(app.clone(), state.clone());
    if !result.new_game_ids.is_empty() {
        let new_games: Vec<Game> = result
            .games
            .iter()
            .filter(|g| result.new_game_ids.contains(&g.id))
            .cloned()
            .collect();
        let _ = app.emit("games-discovered", new_games);
    }
    result
}

#[tauri::command]
pub fn launch_game(app: AppHandle, state: State<'_, Arc<AppState>>, game_id: String) -> Reply<()> {
    let game = {
        let inner = state.inner.lock();
        inner
            .library
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| crate::msg::plain("game_gone"))?
    };

    crate::launch::launch(&game).map_err(|e| e.to_string())?;

    {
        let mut inner = state.inner.lock();
        if let Some(stored) = inner.library.iter_mut().find(|g| g.id == game_id) {
            stored.last_played = Some(gamehub_detect::now_iso8601());
        }
    }
    state.persist_library();
    let _ = app.emit("game-launched", &game_id);
    Ok(())
}

#[tauri::command]
pub fn set_favorite(state: State<'_, Arc<AppState>>, game_id: String, favorite: bool) -> Reply<()> {
    update_game(&state, &game_id, |g| g.favorite = favorite)?;
    state.persist_library();
    Ok(())
}

#[tauri::command]
pub fn set_hidden(state: State<'_, Arc<AppState>>, game_id: String, hidden: bool) -> Reply<()> {
    update_game(&state, &game_id, |g| g.hidden = hidden)?;
    state.persist_library();
    Ok(())
}

#[tauri::command]
pub fn rename_game(state: State<'_, Arc<AppState>>, game_id: String, name: String) -> Reply<()> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 200 {
        return Err(crate::msg::plain("bad_name"));
    }
    update_game(&state, &game_id, |g| {
        g.name = name.clone();
        // Marked so a rescan does not overwrite the user's own name.
        if !g.tags.iter().any(|t| t == "renamed") {
            g.tags.push("renamed".into());
        }
    })?;
    state.persist_library();
    Ok(())
}

#[tauri::command]
pub fn set_tags(state: State<'_, Arc<AppState>>, game_id: String, tags: Vec<String>) -> Reply<()> {
    let cleaned: Vec<String> = tags
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && t.len() <= 40)
        .take(20)
        .collect();
    update_game(&state, &game_id, |g| g.tags = cleaned.clone())?;
    state.persist_library();
    Ok(())
}

fn update_game(state: &State<'_, Arc<AppState>>, game_id: &str, f: impl Fn(&mut Game)) -> Reply<()> {
    let mut inner = state.inner.lock();
    match inner.library.iter_mut().find(|g| g.id == game_id) {
        Some(game) => {
            f(game);
            Ok(())
        }
        None => Err(crate::msg::plain("game_gone")),
    }
}

/// Opens the game's folder in Explorer. The path comes from the library, not
/// from the caller, and is checked to still exist.
#[tauri::command]
pub fn open_game_folder(state: State<'_, Arc<AppState>>, game_id: String) -> Reply<String> {
    let inner = state.inner.lock();
    let game = inner
        .library
        .iter()
        .find(|g| g.id == game_id)
        .ok_or_else(|| crate::msg::plain("game_gone"))?;
    let dir = game
        .install_dir
        .clone()
        .ok_or_else(|| crate::msg::plain("no_install_dir"))?;
    if !Path::new(&dir).is_dir() {
        return Err(crate::msg::plain("folder_gone"));
    }
    Ok(dir)
}

/* -------------------------------------------------------------------------- */
/* Settings                                                                   */
/* -------------------------------------------------------------------------- */

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Settings {
    state.settings()
}

/// Heavy commands run off the main thread.
///
/// A synchronous Tauri command runs on the UI thread, and some of these take a
/// second or more — restarting ffmpeg waits to see that it started, a scan
/// reads nine launchers, a save-folder search walks the user's profile. Run
/// there, they froze every animation in the window for the duration.
async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Reply<T> + Send + 'static) -> Reply<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| crate::msg::code("thread_lost", &[&e.to_string()]))?
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, state: State<'_, Arc<AppState>>, settings: Settings) -> Reply<Settings> {
    let state = state.inner().clone();
    blocking(move || save_settings_sync(&app, &state, settings)).await
}

pub fn save_settings_sync(app: &AppHandle, state: &Arc<AppState>, settings: Settings) -> Reply<Settings> {
    let mut settings = settings;
    settings.scan_interval_minutes = settings.scan_interval_minutes.min(24 * 60);
    settings.extra_game_folders.retain(|f| Path::new(f).is_dir());

    // A background picked from anywhere on disk is copied into GameHub's own
    // folder and referenced there instead. Two reasons: the web view is only
    // allowed to read files it owns, and a background that lives outside would
    // vanish the moment the original was moved, renamed or deleted.
    if !settings.background_image.trim().is_empty() {
        match adopt_background(state, &settings.background_image) {
            Ok(local) => settings.background_image = local,
            Err(error) => eprintln!("keeping the background where it is: {error}"),
        }
    }

    let start_with_windows = settings.start_with_windows;
    settings.replay.buffer_seconds = settings.replay.buffer();
    settings.replay.save_seconds = settings.replay.save_seconds.clamp(5, settings.replay.buffer());
    let (replay_changed, language_changed) = {
        let inner = state.inner.lock();
        (
            serde_json::to_string(&inner.settings.replay).ok() != serde_json::to_string(&settings.replay).ok(),
            inner.settings.language != settings.language,
        )
    };
    {
        let mut inner = state.inner.lock();
        // Privacy and streak settings are held in one place; the activity log
        // reads them from here rather than keeping its own copy.
        inner.activity.tracking_enabled = settings.track_activity;
        inner.activity.streak_threshold_minutes = settings.streak_threshold_minutes;
        inner.settings = settings.clone();
    }
    state.persist_activity();
    state.persist_settings();
    crate::autostart::set(start_with_windows).map_err(|e| e.to_string())?;

    // The screenshot and clip folders may have just changed. Re-granting here
    // means a new folder works straight away instead of after a restart.
    crate::assets::grant(app, state);

    // A changed buffer length, sound source or quality only takes effect by
    // restarting ffmpeg. Done here so the user never has to know that.
    if replay_changed {
        if let Err(error) = apply_replay(app, state) {
            let _ = app.emit("toast", (crate::msg::plain("toast_replay_not_started"), error));
        }
    }

    // The tray menu is the one bit of interface drawn from Rust; it follows
    // the language like everything else.
    if language_changed {
        crate::tray::relabel(app, &settings.language);
    }

    let _ = app.emit("settings-updated", &settings);
    Ok(settings)
}

/// Copies a chosen background into GameHub's own folder and returns the new
/// path. Already-adopted files are left alone rather than copied again.
fn adopt_background(state: &Arc<AppState>, chosen: &str) -> Reply<String> {
    let source = PathBuf::from(chosen);
    let folder = state.paths.backgrounds();

    if source.starts_with(&folder) {
        return Ok(chosen.to_string());
    }
    if !source.is_file() {
        return Err(crate::msg::code("not_a_file", &[&chosen.to_string()]));
    }

    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let extension = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| ["png", "jpg", "jpeg", "webp", "gif", "bmp"].contains(&e.as_str()))
        .ok_or_else(|| crate::msg::plain("not_an_image"))?;

    let name = format!("background-{}.{extension}", gamehub_detect::backup::slug(&gamehub_detect::now_iso8601()));
    let target = gamehub_detect::safepath::join_within(&folder, &name).map_err(|e| e.to_string())?;
    std::fs::copy(&source, &target).map_err(|e| e.to_string())?;

    // Only one background is ever in use; the previous copies are dead weight.
    if let Ok(entries) = std::fs::read_dir(&folder) {
        for entry in entries.flatten() {
            if entry.path() != target {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    Ok(target.display().to_string())
}

/// Adds one game the scanner did not find, from an executable the user picked.
///
/// The path comes from the OS file picker. It is validated the same way every
/// other launch target is — a real file, `.exe`, inside the folder it claims —
/// before it becomes something GameHub will start.
#[tauri::command]
pub fn add_manual_game(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    executable: String,
    name: Option<String>,
) -> Reply<Game> {
    let path = PathBuf::from(&executable);
    let folder = path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| crate::msg::plain("no_folder"))?;
    gamehub_detect::safepath::validate_executable(&path, &[folder.clone()]).map_err(|e| e.to_string())?;

    let title = name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().replace(['_', '-'], " ")))
        .unwrap_or_else(|| "Game".into());

    // The id is derived from the path, so adding the same game twice updates
    // it rather than producing a duplicate.
    let source_id: String = executable
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    let mut game = Game::new(gamehub_detect::model::GameSource::Local, &source_id, title);
    game.install_dir = Some(folder.to_string_lossy().into_owned());
    game.installed = true;
    game.tags.push("added by hand".into());
    game.launch = Some(gamehub_detect::model::LaunchMethod::Executable {
        path: executable.clone(),
        args: Vec::new(),
        working_dir: Some(folder.to_string_lossy().into_owned()),
    });

    {
        let mut inner = state.inner.lock();
        match inner.library.iter_mut().find(|g| g.id == game.id) {
            Some(existing) => *existing = game.clone(),
            None => inner.library.push(game.clone()),
        }
        inner.library.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }
    state.persist_library();
    let _ = app.emit("library-changed", ());
    spawn_artwork_fetch(app, state.inner().clone());
    Ok(game)
}

/// Removes an entry the scanner got wrong.
///
/// A game a launcher still reports would come straight back on the next scan,
/// so it is hidden instead — the effect the user wants, without a rescan
/// undoing it. Hand-added games are deleted outright.
#[tauri::command]
pub fn remove_game(app: AppHandle, state: State<'_, Arc<AppState>>, game_id: String) -> Reply<String> {
    let outcome = {
        let mut inner = state.inner.lock();
        let Some(position) = inner.library.iter().position(|g| g.id == game_id) else {
            return Err(crate::msg::plain("game_gone"));
        };
        let added_by_hand = inner.library[position].tags.iter().any(|t| t == "added by hand");
        if added_by_hand {
            inner.library.remove(position);
            "removed"
        } else {
            inner.library[position].hidden = true;
            "hidden"
        }
    };
    state.persist_library();
    let _ = app.emit("library-changed", ());
    Ok(outcome.to_string())
}

/// Puts back everything that was hidden.
#[tauri::command]
pub fn restore_hidden(app: AppHandle, state: State<'_, Arc<AppState>>) -> Reply<usize> {
    let restored = {
        let mut inner = state.inner.lock();
        let mut count = 0;
        for game in inner.library.iter_mut() {
            if game.hidden {
                game.hidden = false;
                count += 1;
            }
        }
        count
    };
    state.persist_library();
    let _ = app.emit("library-changed", ());
    Ok(restored)
}

#[tauri::command]
pub fn hidden_count(state: State<'_, Arc<AppState>>) -> usize {
    state.inner.lock().library.iter().filter(|g| g.hidden).count()
}

#[tauri::command]
pub fn add_game_folder(state: State<'_, Arc<AppState>>, folder: String) -> Reply<Settings> {
    if !Path::new(&folder).is_dir() {
        return Err(crate::msg::plain("folder_missing"));
    }
    {
        let mut inner = state.inner.lock();
        if !inner.settings.extra_game_folders.contains(&folder) {
            inner.settings.extra_game_folders.push(folder);
        }
    }
    state.persist_settings();
    Ok(state.settings())
}

/* -------------------------------------------------------------------------- */
/* Artwork                                                                    */
/* -------------------------------------------------------------------------- */

/// Fills in missing cover art in the background.
fn spawn_artwork_fetch(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let games = state.inner.lock().library.clone();
        // A cap per pass keeps a first run on a 400-game library from spending
        // several minutes on the network; the rest arrive on the next scan.
        let fetched = crate::artwork::fetch_missing(&games, &state.paths.artwork(), 40);
        if fetched.is_empty() {
            return;
        }
        {
            let mut inner = state.inner.lock();
            for item in &fetched {
                if let Some(game) = inner.library.iter_mut().find(|g| g.id == item.game_id) {
                    game.metadata = Some(item.metadata.clone());
                }
            }
        }
        state.persist_library();
        let _ = app.emit("artwork-updated", fetched.len());
    });
}

#[tauri::command]
pub fn refresh_artwork(app: AppHandle, state: State<'_, Arc<AppState>>) {
    spawn_artwork_fetch(app, state.inner().clone());
}

/// Loads an image the user picked, so the crop tool can show it.
///
/// `path` comes from the OS file picker. It is still validated here: a real
/// file, under the size cap, and an image by signature rather than by name.
#[tauri::command]
pub fn read_image_for_crop(path: String) -> Reply<String> {
    crate::artwork::read_for_crop(Path::new(&path))
}

/// Stores a cropped cover for one game.
///
/// The PNG arrives as a data URL from the crop canvas. Marking it `manual` is
/// what stops the next scan's artwork fetch from replacing it.
#[tauri::command]
pub fn set_custom_cover(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    game_id: String,
    png_data_url: String,
) -> Reply<String> {
    let bytes = crate::artwork::base64_decode(&png_data_url)?;
    let path = crate::artwork::save_custom_cover(&state.paths.artwork(), &game_id, &bytes)?;
    let path_string = path.to_string_lossy().into_owned();

    {
        let mut inner = state.inner.lock();
        let game = inner
            .library
            .iter_mut()
            .find(|g| g.id == game_id)
            .ok_or_else(|| crate::msg::plain("game_gone"))?;
        let mut metadata = game.metadata.clone().unwrap_or_default();
        metadata.cover_path = Some(path_string.clone());
        metadata.provider = Some("manual".into());
        metadata.fetched_at = Some(gamehub_detect::now_iso8601());
        game.metadata = Some(metadata);
    }
    state.persist_library();
    let _ = app.emit("artwork-updated", 1usize);
    Ok(path_string)
}

/// Removes a chosen cover so the game falls back to a fetched one.
#[tauri::command]
pub fn clear_custom_cover(app: AppHandle, state: State<'_, Arc<AppState>>, game_id: String) -> Reply<()> {
    crate::artwork::clear_custom_cover(&state.paths.artwork(), &game_id);
    {
        let mut inner = state.inner.lock();
        if let Some(game) = inner.library.iter_mut().find(|g| g.id == game_id) {
            game.metadata = None;
        }
    }
    state.persist_library();
    let _ = app.emit("artwork-updated", 0usize);
    // The next fetch fills it back in from Steam where one exists.
    spawn_artwork_fetch(app, state.inner().clone());
    Ok(())
}

/* -------------------------------------------------------------------------- */
/* Activity: sessions, streaks, calendar                                      */
/* -------------------------------------------------------------------------- */

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySummary {
    pub streaks: StreakSummary,
    /// Today's totals, or an empty day when nothing has been played.
    pub today: DaySummary,
    /// Game id, when it was last played, and total seconds — newest first.
    pub recent: Vec<(String, String, u64)>,
    /// The session currently in progress, if any. Its `seconds` are active
    /// play so far; `wall_seconds` how long the game has been open.
    pub current: Option<Session>,
    /// Whether the current session's game is being played right now, as
    /// opposed to sitting in the background.
    pub current_active: bool,
    pub tracking_enabled: bool,
}

#[tauri::command]
pub fn get_activity(state: State<'_, Arc<AppState>>) -> ActivitySummary {
    let inner = state.inner.lock();
    let sessions = &inner.activity.sessions;
    let now = time::OffsetDateTime::now_utc();
    let today_key = format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        now.month() as u8,
        now.day()
    );

    let by_day = activity::days(sessions);
    let today = by_day.get(&today_key).cloned().unwrap_or(DaySummary {
        date: today_key,
        ..Default::default()
    });

    // An open session is shown with the time it has run so far, so the Home
    // page can say "Playing — 1h 42m" without waiting for the game to close.
    let current = inner.activity.open.first().map(|open| Session {
        game_id: open.game_id.clone(),
        game_name: open.game_name.clone(),
        started_at: open.started_at.clone(),
        ended_at: gamehub_detect::now_iso8601(),
        seconds: activity::active_so_far(open, now),
        wall_seconds: seconds_since(&open.started_at, now),
    });
    let current_active = inner.activity.open.first().map(|open| open.active).unwrap_or(false);

    ActivitySummary {
        streaks: activity::streaks(sessions, inner.activity.streak_threshold_minutes, now.date()),
        today,
        recent: activity::recently_played(sessions, 8),
        current,
        current_active,
        tracking_enabled: inner.activity.tracking_enabled,
    }
}

fn seconds_since(started_at: &str, now: time::OffsetDateTime) -> u64 {
    time::OffsetDateTime::parse(started_at, &time::format_description::well_known::Rfc3339)
        .map(|start| (now - start).whole_seconds().max(0) as u64)
        .unwrap_or(0)
}

/// Every day with activity in one month, for the calendar. `month` is "YYYY-MM".
#[tauri::command]
pub fn get_calendar_month(state: State<'_, Arc<AppState>>, month: String) -> Vec<DaySummary> {
    let inner = state.inner.lock();
    activity::days(&inner.activity.sessions)
        .into_values()
        .filter(|day| day.date.starts_with(&month))
        .collect()
}

/// The quest board, recomputed from the library and the session log every time
/// it is asked for. Nothing about a quest is stored, so XP cannot be forged.
#[tauri::command]
pub fn get_quests(state: State<'_, Arc<AppState>>) -> gamehub_detect::quests::QuestBoard {
    let inner = state.inner.lock();
    let today = time::OffsetDateTime::now_utc().date();
    gamehub_detect::quests::board(&inner.library, &inner.activity.sessions, today)
}

/// Clears every recorded session. Games, favourites and settings are untouched.
#[tauri::command]
pub fn clear_activity(state: State<'_, Arc<AppState>>) -> Reply<()> {
    {
        let mut inner = state.inner.lock();
        inner.activity.sessions.clear();
        inner.activity.open.clear();
    }
    state.persist_activity();
    Ok(())
}

/* -------------------------------------------------------------------------- */
/* Screenshots                                                                */
/* -------------------------------------------------------------------------- */

/// Takes a screenshot and files it under whatever is being played.
///
/// The game comes from the same running-game watcher that drives playtime and
/// streaks — there is no second guess at what is on screen. Nothing running
/// means the shot is filed under "Desktop" rather than attributed to the last
/// game played.
#[tauri::command]
pub async fn take_screenshot(app: AppHandle, state: State<'_, Arc<AppState>>) -> Reply<gamehub_detect::media::Screenshot> {
    let state = state.inner().clone();
    blocking(move || capture_screenshot(&app, &state)).await
}

pub fn capture_screenshot(
    app: &AppHandle,
    state: &Arc<AppState>,
) -> Reply<gamehub_detect::media::Screenshot> {
    let settings = state.settings();
    let root = state.paths.screenshot_root(&settings.screenshot_folder);
    let current = state.current_game();
    let game_name = current.as_ref().map(|(_, name)| name.as_str()).unwrap_or(gamehub_detect::media::DESKTOP);
    let taken_at = gamehub_detect::now_iso8601();

    let (path, size) = crate::capture::capture(&root, game_name, &taken_at, settings.screenshot_monitor)?;

    let shot = {
        let mut inner = state.inner.lock();
        gamehub_detect::media::add(
            &mut inner.screenshots,
            &path,
            current.as_ref().map(|(id, name)| (id.as_str(), name.as_str())),
            &taken_at,
            size,
        )
    };
    state.persist_screenshots();
    let _ = app.emit("screenshot-taken", &shot);
    Ok(shot)
}

#[tauri::command]
pub fn get_screenshots(state: State<'_, Arc<AppState>>) -> Vec<(String, Vec<gamehub_detect::media::Screenshot>)> {
    let pruned = {
        let mut inner = state.inner.lock();
        gamehub_detect::media::prune_missing(&mut inner.screenshots)
    };
    if pruned > 0 {
        state.persist_screenshots();
    }
    let inner = state.inner.lock();
    gamehub_detect::media::by_game(&inner.screenshots)
}

#[tauri::command]
pub fn set_screenshot_favorite(state: State<'_, Arc<AppState>>, id: String, favorite: bool) -> Reply<()> {
    {
        let mut inner = state.inner.lock();
        gamehub_detect::media::set_favorite(&mut inner.screenshots, &id, favorite);
    }
    state.persist_screenshots();
    Ok(())
}

#[tauri::command]
pub fn delete_screenshot(state: State<'_, Arc<AppState>>, id: String) -> Reply<()> {
    let root = state.paths.screenshot_root(&state.settings().screenshot_folder);
    {
        let mut inner = state.inner.lock();
        gamehub_detect::media::remove(&mut inner.screenshots, &root, &id)?;
    }
    state.persist_screenshots();
    Ok(())
}

#[tauri::command]
pub fn screenshot_folder(state: State<'_, Arc<AppState>>) -> String {
    state
        .paths
        .screenshot_root(&state.settings().screenshot_folder)
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
pub fn list_displays() -> Vec<String> {
    crate::capture::monitor_names()
}

/* -------------------------------------------------------------------------- */
/* Clipboard                                                                  */
/* -------------------------------------------------------------------------- */

#[tauri::command]
pub fn get_clipboard(state: State<'_, Arc<AppState>>, query: Option<String>) -> Vec<gamehub_detect::clipboard::ClipItem> {
    let inner = state.inner.lock();
    gamehub_detect::clipboard::search(&inner.clipboard, query.as_deref().unwrap_or(""))
        .into_iter()
        .cloned()
        .collect()
}

#[tauri::command]
pub fn pin_clip(state: State<'_, Arc<AppState>>, id: String, pinned: bool) -> Reply<()> {
    {
        let mut inner = state.inner.lock();
        gamehub_detect::clipboard::set_pinned(&mut inner.clipboard, &id, pinned);
    }
    state.persist_clipboard();
    Ok(())
}

#[tauri::command]
pub fn delete_clip(state: State<'_, Arc<AppState>>, id: String) -> Reply<()> {
    {
        let mut inner = state.inner.lock();
        gamehub_detect::clipboard::remove(&mut inner.clipboard, &id);
    }
    state.persist_clipboard();
    Ok(())
}

#[tauri::command]
pub fn clear_clipboard(state: State<'_, Arc<AppState>>) -> Reply<()> {
    {
        let mut inner = state.inner.lock();
        gamehub_detect::clipboard::clear(&mut inner.clipboard);
    }
    state.persist_clipboard();
    Ok(())
}

/* -------------------------------------------------------------------------- */
/* Performance                                                                */
/* -------------------------------------------------------------------------- */

#[tauri::command]
pub fn sample_performance(state: State<'_, Arc<AppState>>) -> crate::perf::PerformanceSample {
    let dirs = state.running_game_dirs();
    let mut monitor = state.monitor.lock();
    monitor.sample(&dirs)
}

/* -------------------------------------------------------------------------- */
/* Shortcuts                                                                  */
/* -------------------------------------------------------------------------- */

#[tauri::command]
pub fn get_shortcuts(state: State<'_, Arc<AppState>>) -> Vec<gamehub_detect::shortcuts::Shortcut> {
    state.inner.lock().bindings.list()
}

/// Rebinds one shortcut, refusing a combination another action already holds
/// and naming the one that has it.
#[tauri::command]
pub fn set_shortcut(app: AppHandle, state: State<'_, Arc<AppState>>, action: String, binding: String) -> Reply<String> {
    let action = gamehub_detect::shortcuts::Action::from_id(&action)
        .ok_or_else(|| crate::msg::plain("no_shortcut"))?;
    let result = {
        let mut inner = state.inner.lock();
        gamehub_detect::shortcuts::rebind(&mut inner.bindings, action, &binding).map_err(|e| e.to_string())
    }?;
    state.persist_bindings();
    crate::hotkeys::reregister(&app, state.inner().clone());
    Ok(result)
}

#[tauri::command]
pub fn reset_shortcut(app: AppHandle, state: State<'_, Arc<AppState>>, action: String) -> Reply<()> {
    let action = gamehub_detect::shortcuts::Action::from_id(&action)
        .ok_or_else(|| crate::msg::plain("no_shortcut"))?;
    {
        let mut inner = state.inner.lock();
        gamehub_detect::shortcuts::reset(&mut inner.bindings, action);
    }
    state.persist_bindings();
    crate::hotkeys::reregister(&app, state.inner().clone());
    Ok(())
}

/* -------------------------------------------------------------------------- */
/* Instant replay                                                             */
/* -------------------------------------------------------------------------- */

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayStatus {
    pub enabled: bool,
    pub running: bool,
    /// Why recording stopped, when it is switched on but not running.
    pub problem: Option<String>,
    /// Absent when ffmpeg cannot be found, which is the one thing that stops
    /// replay working at all.
    pub ffmpeg_path: Option<String>,
    pub buffer_seconds: u64,
    /// How many seconds are actually in the buffer right now.
    pub buffered_seconds: u64,
    pub min_buffer_seconds: u64,
    pub max_buffer_seconds: u64,
    pub clip_folder: String,
    /// Rough disk cost of the buffer at the current settings.
    pub buffer_estimate_mb: u64,
    /// What sound the running recording has.
    pub audio: crate::replay::AudioReport,
}

fn ffmpeg_for(app: &AppHandle) -> Option<PathBuf> {
    let resource_dir = app.path().resource_dir().unwrap_or_default();
    crate::replay::find_ffmpeg(&resource_dir)
}

#[tauri::command]
pub fn replay_status(app: AppHandle, state: State<'_, Arc<AppState>>) -> ReplayStatus {
    let settings = state.settings();
    let (running, problem, buffered, audio) = {
        let mut recorder = state.recorder.lock();
        let running = recorder.is_running();
        // Switched on but not recording means ffmpeg died, and it almost
        // always said why. Showing that beats a card that claims to be
        // recording, or one that says "off" with no explanation.
        let problem = (!running && settings.replay.enabled)
            .then(|| recorder.last_complaint())
            .map(|why| why.lines().last().unwrap_or("").trim().to_string())
            .filter(|why| !why.is_empty());
        (running, problem, recorder.seconds_buffered(&settings.replay), recorder.audio.clone())
    };

    ReplayStatus {
        enabled: settings.replay.enabled,
        running,
        problem,
        ffmpeg_path: ffmpeg_for(&app).map(|p| p.to_string_lossy().into_owned()),
        buffer_seconds: settings.replay.buffer(),
        buffered_seconds: buffered,
        min_buffer_seconds: crate::replay::MIN_BUFFER_SECONDS,
        max_buffer_seconds: crate::replay::MAX_BUFFER_SECONDS,
        clip_folder: state
            .paths
            .clip_root(&settings.replay.folder)
            .to_string_lossy()
            .into_owned(),
        buffer_estimate_mb: crate::replay::buffer_estimate_mb(&settings.replay),
        audio,
    }
}

/// Arms or disarms the buffer.
#[tauri::command]
pub async fn set_replay_enabled(app: AppHandle, state: State<'_, Arc<AppState>>, enabled: bool) -> Reply<()> {
    let state = state.inner().clone();
    blocking(move || {
        {
            let mut inner = state.inner.lock();
            inner.settings.replay.enabled = enabled;
        }
        state.persist_settings();
        let result = apply_replay(&app, &state);
        if result.is_err() {
            // Starting failed: the setting must not claim it is on.
            state.inner.lock().settings.replay.enabled = false;
            state.persist_settings();
        }
        let _ = app.emit("settings-updated", &state.settings());
        result
    })
    .await
}

/// Flips Replay on or off and reports the state it landed in.
///
/// Used by the global hotkey, so it can be switched mid-game without alt-tabbing.
pub fn toggle_replay(app: &AppHandle, state: &Arc<AppState>) -> Reply<bool> {
    let wanted = !state.settings().replay.enabled;
    {
        let mut inner = state.inner.lock();
        inner.settings.replay.enabled = wanted;
    }
    state.persist_settings();

    // If starting fails, the setting must not be left claiming it is on.
    if let Err(error) = apply_replay(app, state) {
        let mut inner = state.inner.lock();
        inner.settings.replay.enabled = false;
        drop(inner);
        state.persist_settings();
        let _ = app.emit("settings-updated", &state.settings());
        return Err(error);
    }

    let _ = app.emit("settings-updated", &state.settings());
    Ok(wanted)
}

/// Starts or stops the recorder to match the settings. Called whenever replay
/// settings change, so there is one place that decides what should be running.
pub fn apply_replay(app: &AppHandle, state: &Arc<AppState>) -> Reply<()> {
    let settings = state.settings();
    let mut recorder = state.recorder.lock();

    if !settings.replay.enabled {
        recorder.stop();
        let _ = app.emit("replay-state", false);
        return Ok(());
    }

    let Some(ffmpeg) = ffmpeg_for(app) else {
        return Err(crate::msg::plain("replay_no_ffmpeg"));
    };

    recorder.start(&ffmpeg, &settings.replay)?;
    let _ = app.emit("replay-state", true);
    Ok(())
}

/// Saves the last `seconds` of the buffer as a clip, filed under whatever is
/// being played — the same running-game source screenshots use.
#[tauri::command]
pub async fn save_replay(app: AppHandle, state: State<'_, Arc<AppState>>, seconds: Option<u64>) -> Reply<crate::replay::Clip> {
    let state = state.inner().clone();
    blocking(move || save_replay_now(&app, &state, seconds)).await
}

pub fn save_replay_now(
    app: &AppHandle,
    state: &Arc<AppState>,
    seconds: Option<u64>,
) -> Reply<crate::replay::Clip> {
    let settings = state.settings();
    if !settings.replay.enabled {
        return Err(crate::msg::plain("replay_off"));
    }
    let Some(ffmpeg) = ffmpeg_for(app) else {
        return Err(crate::msg::plain("replay_no_ffmpeg"));
    };

    // The hotkey saves the length the user chose — 30 seconds unless they said
    // otherwise — never the whole buffer just because the buffer is that long.
    let wanted = seconds
        .unwrap_or(settings.replay.save_seconds)
        .clamp(5, settings.replay.buffer());
    let current = state.current_game();
    let game_name = current
        .as_ref()
        .map(|(_, name)| name.as_str())
        .unwrap_or(gamehub_detect::media::DESKTOP);
    let recorded_at = gamehub_detect::now_iso8601();

    let root = state.paths.clip_root(&settings.replay.folder);
    let folder = gamehub_detect::media::folder_for(&root, game_name);
    let destination = gamehub_detect::safepath::join_within(&folder, &crate::replay::clip_name(game_name, &recorded_at))
        .map_err(|e| e.to_string())?;

    let scratch = state.paths.replay_scratch();

    // Before blaming the buffer: if the recorder is not running, the buffer is
    // empty *because* nothing is recording, and ffmpeg almost certainly said
    // why. Reporting "there is nothing in the buffer" here describes the
    // symptom and hides the cause, which is how a broken encoder looked for a
    // whole release like a buffer that just would not fill.
    let has_audio = {
        let mut recorder = state.recorder.lock();
        if !recorder.is_running() {
            let why = recorder.last_complaint();
            let last = why.lines().last().unwrap_or("").trim().to_string();
            return Err(if last.is_empty() {
                crate::msg::plain("replay_not_running")
            } else {
                crate::msg::code("replay_stopped", &[&last])
            });
        }
        recorder.audio.system_audio || recorder.audio.microphone
    };

    let size = crate::replay::save_clip(&ffmpeg, &scratch, &destination, wanted)?;

    let clip = crate::replay::Clip {
        id: format!("{recorded_at}-clip"),
        path: destination.to_string_lossy().into_owned(),
        game_id: current.as_ref().map(|(id, _)| id.clone()),
        game_name: game_name.to_string(),
        recorded_at,
        seconds: wanted,
        size_bytes: size,
        favorite: false,
        has_audio,
    };

    {
        let mut inner = state.inner.lock();
        inner.clips.clips.insert(0, clip.clone());
    }
    state.persist_clips();
    let _ = app.emit("clip-saved", &clip);
    Ok(clip)
}

#[tauri::command]
pub fn get_clips(state: State<'_, Arc<AppState>>) -> Vec<crate::replay::Clip> {
    let pruned = {
        let mut inner = state.inner.lock();
        let before = inner.clips.clips.len();
        inner.clips.clips.retain(|c| Path::new(&c.path).is_file());
        before != inner.clips.clips.len()
    };
    if pruned {
        state.persist_clips();
    }
    state.inner.lock().clips.clips.clone()
}

#[tauri::command]
pub fn delete_replay_clip(state: State<'_, Arc<AppState>>, id: String) -> Reply<()> {
    let root = state.paths.clip_root(&state.settings().replay.folder);
    {
        let mut inner = state.inner.lock();
        let Some(position) = inner.clips.clips.iter().position(|c| c.id == id) else {
            return Err(crate::msg::plain("clip_gone"));
        };
        let path = PathBuf::from(&inner.clips.clips[position].path);
        if !gamehub_detect::safepath::is_within(&root, &path) {
            return Err(crate::msg::plain("clip_outside"));
        }
        let _ = std::fs::remove_file(&path);
        inner.clips.clips.remove(position);
    }
    state.persist_clips();
    Ok(())
}

#[tauri::command]
pub fn set_clip_favorite(state: State<'_, Arc<AppState>>, id: String, favorite: bool) -> Reply<()> {
    {
        let mut inner = state.inner.lock();
        if let Some(clip) = inner.clips.clips.iter_mut().find(|c| c.id == id) {
            clip.favorite = favorite;
        }
    }
    state.persist_clips();
    Ok(())
}

// ---------------------------------------------------------------------------
// Your data
//
// Everything in this section exists so that updating GameHub can never cost
// somebody the work they put into their library — the games they added by hand,
// the ones they hid, the covers they uploaded, the screenshots they kept.
// ---------------------------------------------------------------------------

/// Every backup on disk, newest first.
#[tauri::command]
pub fn list_backups(state: State<'_, Arc<AppState>>) -> Vec<gamehub_detect::backup::Snapshot> {
    gamehub_detect::backup::list_snapshots(&state.paths.data_dir)
}

/// Anything that had to be recovered while starting.
///
/// Normally empty. When it is not, the user is shown what happened rather than
/// left to work out for themselves why something looks different.
#[tauri::command]
pub fn get_recoveries(state: State<'_, Arc<AppState>>) -> Vec<crate::store::RecoveryNote> {
    state.paths.recoveries()
}

/// Takes a backup right now, on request.
#[tauri::command]
pub fn backup_now(app: AppHandle, state: State<'_, Arc<AppState>>) -> Reply<Option<gamehub_detect::backup::Snapshot>> {
    let now = gamehub_detect::now_iso8601();
    let version = app.package_info().version.to_string();
    let id = format!("{}-manual", gamehub_detect::backup::slug(&now));

    let snapshot = gamehub_detect::backup::take_snapshot(
        &state.paths.data_dir,
        &id,
        "manual",
        &version,
        &now,
    )
    .map_err(|e| crate::msg::code("backup_write", &[&e.to_string()]))?;

    gamehub_detect::backup::prune(&state.paths.data_dir, 10);
    Ok(snapshot)
}

/// Puts a backup back.
///
/// The current state is backed up first, so choosing the wrong one is not a
/// one-way door. The app is then reloaded from disk so the window shows the
/// restored data without needing a restart.
#[tauri::command]
pub fn restore_backup(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Reply<Vec<String>> {
    let now = gamehub_detect::now_iso8601();
    let version = app.package_info().version.to_string();
    let safety = format!("{}-before-restore", gamehub_detect::backup::slug(&now));

    let restored = gamehub_detect::backup::restore_snapshot(
        &state.paths.data_dir,
        &id,
        &safety,
        &version,
        &now,
    )
    .map_err(|e| crate::msg::code("backup_restore", &[&e.to_string()]))?;

    // Read the restored files back into memory.
    let paths = &state.paths;
    let settings = crate::store::load_settings(paths);
    let library = crate::store::load_library(paths);
    let mut activity = crate::store::load_activity(paths);
    activity.tracking_enabled = settings.track_activity;
    activity.streak_threshold_minutes = settings.streak_threshold_minutes;
    let screenshots = crate::store::load_screenshots(paths);
    let clips = crate::store::load_clips(paths);
    let bindings = crate::store::load_bindings(paths);

    {
        let mut inner = state.inner.lock();
        inner.settings = settings;
        inner.library = library;
        inner.activity = activity;
        inner.screenshots = screenshots;
        inner.clips = clips;
        inner.bindings = bindings;
    }

    // Shortcuts and folder permissions both came out of the restored files.
    let handle = app.clone();
    let inner_state = state.inner().clone();
    crate::hotkeys::reregister(&handle, inner_state.clone());
    crate::assets::grant(&handle, &inner_state);

    let _ = app.emit("library-updated", ());
    let _ = app.emit("settings-updated", &state.settings());
    Ok(restored)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFolders {
    pub data: String,
    /// Deliberately beside the data folder rather than inside it, so an
    /// uninstall or a botched update cannot take the backups with it.
    pub backups: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOptions {
    pub devices: Vec<String>,
    /// The one that captures what you hear, when the machine has one.
    pub suggested: Option<String>,
}

/// The microphones Replay can mix in.
///
/// System audio no longer needs a device at all — it comes through WASAPI
/// loopback. This list is for a microphone. Offered as a list rather than a
/// text box because the name has to match exactly, brackets and all.
#[tauri::command]
pub fn replay_audio_devices(app: AppHandle) -> AudioOptions {
    let Some(ffmpeg) = ffmpeg_for(&app) else {
        return AudioOptions { devices: Vec::new(), suggested: None };
    };
    let devices = crate::replay::audio_devices(&ffmpeg);
    let suggested = crate::replay::pick_loopback(&devices);
    AudioOptions { devices, suggested }
}

/// Where GameHub keeps everything, so the user can find it in Explorer.
#[tauri::command]
pub fn data_folder(state: State<'_, Arc<AppState>>) -> DataFolders {
    DataFolders {
        data: state.paths.data_dir.display().to_string(),
        backups: gamehub_detect::backup::backups_dir(&state.paths.data_dir)
            .display()
            .to_string(),
    }
}

/// True when this build was made with an updater key and can update itself.
#[tauri::command]
pub fn updates_supported() -> bool {
    crate::state::updates_available()
}

/* -------------------------------------------------------------------------- */
/* Freeze points                                                              */
/* -------------------------------------------------------------------------- */

use gamehub_detect::freeze::{self, FreezeLibrary, FreezePoint, FreezeState};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezeStatus {
    /// False on a machine where Windows will not let processes be suspended.
    pub supported: bool,
    /// The game currently running, if any — what F7 would freeze.
    pub current_game_id: Option<String>,
    pub current_game_name: Option<String>,
    /// The freeze point holding the current game, if it is frozen now.
    pub active: Option<FreezePoint>,
    /// The confirmed save folder for the current game, when there is one.
    pub save_dir: Option<String>,
    pub folder: String,
}

fn clip_root(state: &AppState) -> PathBuf {
    state.paths.clip_root(&state.settings().replay.folder)
}

#[tauri::command]
pub fn freeze_status(state: State<'_, Arc<AppState>>) -> FreezeStatus {
    let current = state.current_game();
    let inner = state.inner.lock();
    let active = current
        .as_ref()
        .and_then(|(id, _)| freeze::active_for(&inner.freezes, id).cloned());
    let save_dir = current.as_ref().and_then(|(id, _)| inner.freezes.save_dirs.get(id).cloned());
    FreezeStatus {
        supported: crate::freeze::supported(),
        current_game_id: current.as_ref().map(|(id, _)| id.clone()),
        current_game_name: current.as_ref().map(|(_, name)| name.clone()),
        active,
        save_dir,
        folder: state.paths.clip_root(&inner.settings.replay.folder).to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub fn get_freezes(state: State<'_, Arc<AppState>>) -> Vec<FreezePoint> {
    let changed = {
        let mut inner = state.inner.lock();
        freeze::reconcile(&mut inner.freezes, crate::freeze::is_alive)
    };
    if changed > 0 {
        state.persist_freezes();
    }
    state.inner.lock().freezes.points.clone()
}

/// Freezes the running game, or the one named, right now.
///
/// Three things happen, in this order, and each is reported separately so a
/// failure in one never hides the others: the processes are suspended (the
/// part that cannot wait), the screen is photographed, and the save folder —
/// if one is known for the game — is copied into the freeze point's folder.
#[tauri::command]
pub async fn freeze_now(app: AppHandle, state: State<'_, Arc<AppState>>, game_id: Option<String>) -> Reply<FreezePoint> {
    let state = state.inner().clone();
    blocking(move || freeze_game(&app, &state, game_id)).await
}

pub fn freeze_game(app: &AppHandle, state: &Arc<AppState>, game_id: Option<String>) -> Reply<FreezePoint> {
    if !crate::freeze::supported() {
        return Err(crate::msg::plain("freeze_unsupported"));
    }

    // Which game: the one asked for, else whatever is being played.
    let (game_id, game_name, install_dir) = {
        let inner = state.inner.lock();
        let id = match game_id {
            Some(id) => id,
            None => inner
                .activity
                .open
                .first()
                .map(|open| open.game_id.clone())
                .or_else(|| inner.running.first().cloned())
                .ok_or_else(|| crate::msg::plain("no_game_running"))?,
        };
        let game = inner
            .library
            .iter()
            .find(|g| g.id == id)
            .ok_or_else(|| crate::msg::plain("game_gone"))?;
        if freeze::active_for(&inner.freezes, &id).is_some() {
            return Err(crate::msg::code("already_frozen", &[&game.name]));
        }
        let dir = game
            .install_dir
            .clone()
            .ok_or_else(|| crate::msg::plain("no_install_dir_freeze"))?;
        (id, game.name.clone(), PathBuf::from(dir))
    };

    let pids = crate::freeze::pids_for(&install_dir);
    if pids.is_empty() {
        return Err(crate::msg::code("game_not_running", &[&game_name]));
    }

    // 1. Stop it. This is the part that must happen first and fast.
    crate::freeze::suspend_all(&pids)?;

    let frozen_at = gamehub_detect::now_iso8601();
    let root = clip_root(state);
    let folder = freeze::folder_for(&root, &game_name, &frozen_at);
    std::fs::create_dir_all(&folder).map_err(|e| crate::msg::code("freeze_folder", &[&e.to_string()]))?;

    let mut point = FreezePoint {
        id: format!("{frozen_at}-freeze"),
        game_id: Some(game_id.clone()),
        game_name: game_name.clone(),
        frozen_at: frozen_at.clone(),
        state: FreezeState::Frozen,
        pids,
        folder: folder.to_string_lossy().into_owned(),
        save_dir: None,
        save_copy: None,
        save_files: 0,
        save_bytes: 0,
        skipped: Vec::new(),
        picture: None,
        note: String::new(),
    };

    // 2. A picture of the moment, so the list is recognisable. A game in
    //    exclusive fullscreen gives a blank frame; that is simply no picture.
    let monitor = state.settings().screenshot_monitor;
    if let Ok((path, _)) = crate::capture::capture(&folder, "frame", &frozen_at, monitor) {
        // capture() files under a subfolder per "game"; the freeze folder is
        // the game here, so move the picture up beside the save copy.
        let target = folder.join("bilde.png");
        if std::fs::rename(&path, &target).is_ok() {
            let _ = std::fs::remove_dir(path.parent().unwrap_or(&folder));
            point.picture = Some(target.to_string_lossy().into_owned());
        } else {
            point.picture = Some(path.to_string_lossy().into_owned());
        }
    }

    // 3. The save copy, when the game's save folder is known.
    let save_dir = state.inner.lock().freezes.save_dirs.get(&game_id).cloned();
    if let Some(save_dir) = save_dir.filter(|d| Path::new(d).is_dir()) {
        let copy = folder.join("save");
        match gamehub_detect::saves::copy_tree(Path::new(&save_dir), &copy) {
            Ok(report) => {
                point.save_dir = Some(save_dir);
                freeze::record_copy(&mut point, &copy, &report);
            }
            Err(error) => point.note = crate::msg::code("save_copy_failed", &[&error.to_string()]),
        }
    }

    let _ = std::fs::write(folder.join("frys.json"), serde_json::to_vec_pretty(&point).unwrap_or_default());

    {
        let mut inner = state.inner.lock();
        inner.freezes.points.insert(0, point.clone());
    }
    state.persist_freezes();
    let _ = app.emit("freeze-changed", &point);
    Ok(point)
}

/// Lets a frozen game continue.
#[tauri::command]
pub async fn resume_freeze(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Reply<FreezePoint> {
    let state = state.inner().clone();
    blocking(move || resume_point(&app, &state, &id)).await
}

pub fn resume_point(app: &AppHandle, state: &Arc<AppState>, id: &str) -> Reply<FreezePoint> {
    let pids = {
        let inner = state.inner.lock();
        let point = inner
            .freezes
            .points
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| crate::msg::plain("freeze_gone"))?;
        if point.state != FreezeState::Frozen {
            return Err(crate::msg::plain("not_frozen"));
        }
        point.pids.clone()
    };

    let resumed = crate::freeze::resume_all(&pids)?;
    let point = {
        let mut inner = state.inner.lock();
        let point = inner.freezes.points.iter_mut().find(|p| p.id == id).expect("checked above");
        point.state = if resumed > 0 { FreezeState::Resumed } else { FreezeState::Gone };
        point.clone()
    };
    state.persist_freezes();
    let _ = app.emit("freeze-changed", &point);
    if resumed == 0 {
        return Err(crate::msg::plain("resume_nothing"));
    }
    Ok(point)
}

/// What the hotkey does: freeze the running game, or resume it if frozen.
/// Returns whether it is now frozen, and the game's name.
pub fn toggle_freeze(app: &AppHandle, state: &Arc<AppState>) -> Reply<(bool, String)> {
    let active = {
        let inner = state.inner.lock();
        inner
            .freezes
            .points
            .iter()
            .find(|p| p.state == FreezeState::Frozen)
            .map(|p| (p.id.clone(), p.game_name.clone()))
    };
    match active {
        Some((id, name)) => {
            resume_point(app, state, &id)?;
            Ok((false, name))
        }
        None => {
            let point = freeze_game(app, state, None)?;
            Ok((true, point.game_name))
        }
    }
}

/// Puts a freeze point's save copy back into the game's save folder.
///
/// The current contents are copied aside first, into the same freeze folder,
/// so restoring the wrong point is never a one-way door.
#[tauri::command]
pub async fn restore_freeze_save(state: State<'_, Arc<AppState>>, id: String) -> Reply<String> {
    let state = state.inner().clone();
    blocking(move || restore_freeze_save_sync(&state, &id)).await
}

fn restore_freeze_save_sync(state: &Arc<AppState>, id: &str) -> Reply<String> {
    let (copy, save_dir, folder, game_id) = {
        let inner = state.inner.lock();
        let point = inner
            .freezes
            .points
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| crate::msg::plain("freeze_gone"))?;
        let copy = point
            .save_copy
            .clone()
            .ok_or_else(|| crate::msg::plain("no_save_copy"))?;
        // The game's *current* save folder wins over the one recorded at the
        // time, in case it was corrected since.
        let save_dir = point
            .game_id
            .as_ref()
            .and_then(|gid| inner.freezes.save_dirs.get(gid).cloned())
            .or_else(|| point.save_dir.clone())
            .ok_or_else(|| crate::msg::plain("no_save_dir"))?;
        (copy, save_dir, point.folder.clone(), point.game_id.clone())
    };

    // Restoring under a running game means the game overwrites it again on
    // its next save, or worse, reads half of it.
    if let Some(gid) = &game_id {
        if state.inner.lock().running.contains(gid) {
            return Err(crate::msg::plain("close_game_first"));
        }
    }

    let stamp = gamehub_detect::backup::slug(&gamehub_detect::now_iso8601());
    let aside = Path::new(&folder).join(format!("before-restore-{stamp}"));
    if Path::new(&save_dir).is_dir() {
        gamehub_detect::saves::copy_tree(Path::new(&save_dir), &aside)
            .map_err(|e| crate::msg::code("aside_failed", &[&e.to_string()]))?;
    }
    let report = gamehub_detect::saves::copy_tree(Path::new(&copy), Path::new(&save_dir))
        .map_err(|e| crate::msg::code("restore_failed", &[&e.to_string()]))?;
    Ok(crate::msg::code(
        "restore_done",
        &[&report.files.to_string(), &save_dir, &aside.display().to_string()],
    ))
}

#[tauri::command]
pub fn delete_freeze(app: AppHandle, state: State<'_, Arc<AppState>>, id: String) -> Reply<()> {
    let root = clip_root(&state);
    {
        let mut inner = state.inner.lock();
        if let Some(point) = inner.freezes.points.iter().find(|p| p.id == id) {
            if point.state == FreezeState::Frozen {
                return Err(crate::msg::plain("resume_before_delete"));
            }
        }
        freeze::remove(&mut inner.freezes, &root, &id)?;
    }
    state.persist_freezes();
    let _ = app.emit("freeze-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_freeze_note(state: State<'_, Arc<AppState>>, id: String, note: String) -> Reply<()> {
    {
        let mut inner = state.inner.lock();
        let point = inner
            .freezes
            .points
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| crate::msg::plain("freeze_gone"))?;
        point.note = note.trim().chars().take(400).collect();
    }
    state.persist_freezes();
    Ok(())
}

/// Where a game's saves might be, for the user to confirm.
#[tauri::command]
pub async fn find_save_folders(state: State<'_, Arc<AppState>>, game_id: String) -> Reply<Vec<gamehub_detect::saves::SaveCandidate>> {
    let state = state.inner().clone();
    blocking(move || find_save_folders_sync(&state, &game_id)).await
}

fn find_save_folders_sync(state: &Arc<AppState>, game_id: &str) -> Reply<Vec<gamehub_detect::saves::SaveCandidate>> {
    let (name, install_dir) = {
        let inner = state.inner.lock();
        let game = inner
            .library
            .iter()
            .find(|g| g.id == game_id)
            .ok_or_else(|| crate::msg::plain("game_gone"))?;
        (game.name.clone(), game.install_dir.clone().map(PathBuf::from))
    };
    Ok(gamehub_detect::saves::find_candidates(&state.env.folders, &name, install_dir.as_deref()))
}

/// The save folder confirmed for a game, if any.
#[tauri::command]
pub fn get_save_folder(state: State<'_, Arc<AppState>>, game_id: String) -> Option<String> {
    state.inner.lock().freezes.save_dirs.get(&game_id).cloned()
}

/// Records the save folder for a game. Empty clears it.
#[tauri::command]
pub fn set_save_folder(state: State<'_, Arc<AppState>>, game_id: String, folder: String) -> Reply<()> {
    let folder = folder.trim().to_string();
    if !folder.is_empty() && !Path::new(&folder).is_dir() {
        return Err(crate::msg::plain("folder_missing"));
    }
    {
        let mut inner = state.inner.lock();
        if folder.is_empty() {
            inner.freezes.save_dirs.remove(&game_id);
        } else {
            inner.freezes.save_dirs.insert(game_id, folder);
        }
    }
    state.persist_freezes();
    Ok(())
}

#[allow(dead_code)]
fn _freeze_library_type_is_used(_: &FreezeLibrary) {}

// ------------------------------------------------------------------ wallpapers

#[tauri::command]
pub async fn wallpaper_status(state: State<'_, Arc<AppState>>) -> Reply<crate::wallpaper::WallpaperStatus> {
    let state = state.inner().clone();
    // Talks to COM, so off the UI thread like everything else that can wait.
    blocking(move || Ok(crate::wallpaper::status(&state))).await
}

/// Sets a picture as the desktop background (every monitor, or one by its
/// id) or the lock screen picture, and returns the new state.
#[tauri::command]
pub async fn set_wallpaper(
    state: State<'_, Arc<AppState>>,
    target: String,
    path: String,
    monitor: Option<String>,
) -> Reply<crate::wallpaper::WallpaperStatus> {
    let state = state.inner().clone();
    blocking(move || {
        let target = crate::wallpaper::Target::parse(&target)?;
        crate::wallpaper::apply(&state, target, Path::new(&path), monitor.as_deref())?;
        Ok(crate::wallpaper::status(&state))
    })
    .await
}

/// Forgets GameHub's choice without touching what Windows shows.
#[tauri::command]
pub async fn forget_wallpaper(
    state: State<'_, Arc<AppState>>,
    target: String,
    monitor: Option<String>,
) -> Reply<crate::wallpaper::WallpaperStatus> {
    let state = state.inner().clone();
    blocking(move || {
        let target = crate::wallpaper::Target::parse(&target)?;
        crate::wallpaper::forget(&state, target, monitor.as_deref())?;
        Ok(crate::wallpaper::status(&state))
    })
    .await
}

/// Puts Windows' own default picture back.
#[tauri::command]
pub async fn default_wallpaper(
    state: State<'_, Arc<AppState>>,
    target: String,
    monitor: Option<String>,
) -> Reply<crate::wallpaper::WallpaperStatus> {
    let state = state.inner().clone();
    blocking(move || {
        let target = crate::wallpaper::Target::parse(&target)?;
        crate::wallpaper::restore_default(&state, target, monitor.as_deref())?;
        Ok(crate::wallpaper::status(&state))
    })
    .await
}
