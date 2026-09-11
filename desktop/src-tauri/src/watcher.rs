//! Noticing a new game without burning the CPU.
//!
//! Two mechanisms, both cheap. A debounced filesystem watcher on the handful of
//! folders launchers write to when they install something — Steam's
//! `steamapps`, Epic's manifest directory, and so on — and a slow timer as a
//! backstop for the launchers that write somewhere else. Neither ever walks a
//! whole drive.

use std::sync::Arc;
use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

/// Long enough that installing a game — which writes hundreds of files — causes
/// one scan rather than hundreds.
const DEBOUNCE: Duration = Duration::from_secs(8);

pub fn spawn(app: AppHandle, state: Arc<AppState>) {
    spawn_watchers(app.clone(), state.clone());
    spawn_timer(app.clone(), state.clone());
    spawn_process_poll(app, state);
}

fn spawn_watchers(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let targets = gamehub_adapters::scanner::watch_targets(&state.env, &state.extra_folders());
        if targets.is_empty() {
            return;
        }

        let handler_state = state.clone();
        let handler_app = app.clone();
        let Ok(mut debouncer) = new_debouncer(DEBOUNCE, move |result: DebounceEventResult| {
            if result.is_ok() {
                crate::commands::run_scan(&handler_app, &handler_state);
            }
        }) else {
            return;
        };

        for target in &targets {
            // A folder that disappears (an unplugged drive) must not take the
            // watcher down with it.
            let _ = debouncer.watcher().watch(target, RecursiveMode::NonRecursive);
        }
        let _ = app.emit("watchers-started", targets.len());

        // The debouncer stops when it is dropped, so this thread parks forever.
        loop {
            std::thread::sleep(Duration::from_secs(3600));
        }
    });
}

fn spawn_timer(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || loop {
        let minutes = state.settings().scan_interval_minutes;
        if minutes == 0 {
            std::thread::sleep(Duration::from_secs(300));
            continue;
        }
        std::thread::sleep(Duration::from_secs(minutes * 60));
        crate::commands::run_scan(&app, &state);
    });
}

/// Polls for running games. Ten seconds is frequent enough for the Play button
/// to feel live and rare enough to be invisible in Task Manager.
///
/// This is also the one place that decides a game is being played. Sessions,
/// playtime, streaks and the calendar are all derived from what this loop sees,
/// so nothing else in GameHub tries to work out whether a game is running.
fn spawn_process_poll(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut watch = crate::process::ProcessWatch::new();
        loop {
            std::thread::sleep(Duration::from_secs(10));
            let library = state.inner.lock().library.clone();
            if library.is_empty() {
                continue;
            }
            let settings = state.settings();
            let idle_limit = (settings.idle_minutes > 0).then(|| Duration::from_secs(settings.idle_minutes * 60));
            let seen = watch.observe(&library, idle_limit);
            let running: Vec<String> = seen.iter().map(|(id, _)| id.clone()).collect();

            let (changed, closed) = {
                let mut inner = state.inner.lock();
                // Names travel with the ids so a session survives the game
                // later being renamed or uninstalled. A frozen game is never
                // being played, whatever window is in front.
                let named: Vec<gamehub_detect::activity::Running> = seen
                    .iter()
                    .filter_map(|(id, active)| {
                        let game = library.iter().find(|g| &g.id == id)?;
                        let frozen = gamehub_detect::freeze::active_for(&inner.freezes, id).is_some();
                        Some(gamehub_detect::activity::Running {
                            game_id: game.id.clone(),
                            game_name: game.name.clone(),
                            active: (*active || !settings.track_active_only) && !frozen,
                        })
                    })
                    .collect();
                let changed = inner.running != running;
                inner.running = running.clone();
                let now = time::OffsetDateTime::now_utc();
                let closed = gamehub_detect::activity::observe_active(&mut inner.activity, &named, now);
                (changed, closed)
            };

            if !closed.is_empty() {
                state.persist_activity();
                for session in &closed {
                    let _ = app.emit("session-ended", &session.0);
                }
                let _ = app.emit("activity-updated", ());
            }
            if changed {
                let _ = app.emit("running-games", running);
                // A session opening is worth persisting too: a power cut should
                // not lose the fact that a game was started.
                state.persist_activity();
            }
        }
    });
}
