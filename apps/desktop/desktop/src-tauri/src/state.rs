//! Everything the app holds in memory, behind one lock.

use std::path::PathBuf;
use std::sync::Arc;

use gamehub_detect::{
    activity::Activity, clipboard::ClipboardHistory, media::ScreenshotLibrary,
    model::Game, shortcuts::Bindings, Env,
};
use parking_lot::Mutex;

use crate::store::{self, Paths, Settings};

pub struct AppState {
    pub paths: Paths,
    pub env: Env,
    pub inner: Mutex<Inner>,
    /// Held across samples so CPU percentages are deltas rather than zero.
    pub monitor: Mutex<crate::perf::PerformanceMonitor>,
    /// The rolling replay buffer, when it is armed.
    pub recorder: Mutex<crate::replay::Recorder>,
}

pub struct Inner {
    pub library: Vec<Game>,
    pub settings: Settings,
    pub scanning: bool,
    /// Ids of games currently running, refreshed by the process poller.
    pub running: Vec<String>,
    /// Sessions, open and closed. The single source of truth for playtime,
    /// streaks and the calendar.
    pub activity: Activity,
    pub clipboard: ClipboardHistory,
    pub screenshots: ScreenshotLibrary,
    pub bindings: Bindings,
    pub clips: crate::replay::ClipLibrary,
    pub freezes: gamehub_detect::freeze::FreezeLibrary,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Arc<Self> {
        let paths = Paths::new(data_dir);
        // Everything is loaded through the one `paths`, so any recovery note a
        // load produces lands in the same place the UI reads from. The previous
        // code built throwaway copies here, which would have silently dropped
        // those notes on the floor.
        let scratch = paths.replay_scratch();

        // Everything is read before `paths` is moved into the struct, and read
        // through that one `paths` — so a file that has to be recovered records
        // its note where the UI will later look for it.
        let settings = store::load_settings(&paths);
        let library = store::load_library(&paths);
        let mut activity = store::load_activity(&paths);
        activity.tracking_enabled = settings.track_activity;
        activity.streak_threshold_minutes = settings.streak_threshold_minutes;

        let mut clipboard = store::load_clipboard(&paths);
        clipboard.enabled = settings.clipboard_enabled;
        let screenshots = store::load_screenshots(&paths);
        let bindings = store::load_bindings(&paths);
        let clips = store::load_clips(&paths);
        let freezes = store::load_freezes(&paths);

        Arc::new(Self {
            paths,
            env: Env::system(),
            monitor: Mutex::new(crate::perf::PerformanceMonitor::new()),
            recorder: Mutex::new(crate::replay::Recorder::new(scratch)),
            inner: Mutex::new(Inner {
                clipboard,
                screenshots,
                bindings,
                clips,
                freezes,
                library,
                settings,
                scanning: false,
                running: Vec::new(),
                activity,
            }),
        })
    }

    pub fn settings(&self) -> Settings {
        self.inner.lock().settings.clone()
    }

    pub fn extra_folders(&self) -> Vec<PathBuf> {
        self.inner
            .lock()
            .settings
            .extra_game_folders
            .iter()
            .map(PathBuf::from)
            .collect()
    }

    pub fn persist_library(&self) {
        let library = self.inner.lock().library.clone();
        let _ = store::save_library(&self.paths, &library);
    }

    pub fn persist_settings(&self) {
        let settings = self.inner.lock().settings.clone();
        let _ = store::save_settings(&self.paths, &settings);
    }

    pub fn persist_activity(&self) {
        let activity = self.inner.lock().activity.clone();
        let _ = store::save_activity(&self.paths, &activity);
    }

    pub fn persist_clipboard(&self) {
        let history = self.inner.lock().clipboard.clone();
        let _ = store::save_clipboard(&self.paths, &history);
    }

    pub fn persist_screenshots(&self) {
        let library = self.inner.lock().screenshots.clone();
        let _ = store::save_screenshots(&self.paths, &library);
    }

    pub fn persist_clips(&self) {
        let library = self.inner.lock().clips.clone();
        let _ = store::save_clips(&self.paths, &library);
    }

    pub fn persist_freezes(&self) {
        let library = self.inner.lock().freezes.clone();
        let _ = store::save_freezes(&self.paths, &library);
    }

    pub fn persist_bindings(&self) {
        let bindings = self.inner.lock().bindings.clone();
        let _ = store::save_bindings(&self.paths, &bindings);
    }

    /// The game currently being played, if any — the one source every feature
    /// asks when it needs to know what to attribute something to.
    pub fn current_game(&self) -> Option<(String, String)> {
        let inner = self.inner.lock();
        inner
            .activity
            .open
            .first()
            .map(|open| (open.game_id.clone(), open.game_name.clone()))
    }

    /// Install folders of whatever is running, for the performance page.
    pub fn running_game_dirs(&self) -> Vec<String> {
        let inner = self.inner.lock();
        inner
            .library
            .iter()
            .filter(|g| inner.running.contains(&g.id))
            .filter_map(|g| g.install_dir.clone())
            .collect()
    }
}

/// Whether this build can update itself.
///
/// A build made on your own PC has no signing key, so the updater plugin is not
/// configured and never registers. The UI asks this so it can say so plainly
/// rather than offering a button that always fails.
static UPDATES_AVAILABLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_updates_available(value: bool) {
    UPDATES_AVAILABLE.store(value, std::sync::atomic::Ordering::Relaxed);
}

pub fn updates_available() -> bool {
    UPDATES_AVAILABLE.load(std::sync::atomic::Ordering::Relaxed)
}
