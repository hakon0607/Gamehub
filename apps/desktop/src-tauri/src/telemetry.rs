//! Anonymous usage statistics for the GameHub website's admin page.
//!
//! Every five minutes while GameHub runs (and once at start) the app sends
//! one small JSON message to the website: a random install id made on first
//! run, the version, the language, whether the window is on screen or in
//! the tray, the game being played right now, how many games are installed
//! and from which launchers, the Windows version, and how many times each
//! feature was used since the last message. No name, no account, no file
//! paths, no IP stored — the website keeps the country GitHub's hosting
//! derives from the request and drops the address.
//!
//! Nothing here can break the app: every failure is swallowed, and counts
//! that could not be delivered are carried over to the next attempt.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Where the messages go. Overridable for a local test of the website.
const STATS_URL: &str = "https://webgamehubweb.vercel.app/api/ping";
/// How often a running app says hello. The website treats anything heard
/// from in the last ten minutes as "active now".
pub const INTERVAL: Duration = Duration::from_secs(5 * 60);

/// The features worth counting, by the name the website shows.
pub const EVENTS: [&str; 7] = ["launch", "screenshot", "clip", "trim", "freeze", "wallpaper", "quicktools"];
static COUNTS: [AtomicU32; EVENTS.len()] = [const { AtomicU32::new(0) }; EVENTS.len()];

/// Notes one use of a feature. Cheap enough to call from anywhere.
pub fn hit(event: &str) {
    if let Some(i) = EVENTS.iter().position(|e| *e == event) {
        COUNTS[i].fetch_add(1, Ordering::Relaxed);
    }
}

/// Takes the counts since the last message, leaving zeros behind.
fn take_counts() -> Vec<(&'static str, u32)> {
    EVENTS
        .iter()
        .enumerate()
        .map(|(i, name)| (*name, COUNTS[i].swap(0, Ordering::Relaxed)))
        .filter(|(_, n)| *n > 0)
        .collect()
}

/// Puts undelivered counts back so the next message carries them.
fn restore_counts(counts: &[(&'static str, u32)]) {
    for (name, n) in counts {
        if let Some(i) = EVENTS.iter().position(|e| e == name) {
            COUNTS[i].fetch_add(*n, Ordering::Relaxed);
        }
    }
}

/// The message itself. Pure, so the tests can look at it.
pub fn payload(
    install_id: &str,
    version: &str,
    language: &str,
    on_screen: bool,
    playing: Option<&str>,
    games: usize,
    launchers: &[String],
    os: &str,
    events: &[(&'static str, u32)],
) -> serde_json::Value {
    serde_json::json!({
        "id": install_id,
        "version": version,
        "language": language,
        "state": if on_screen { "open" } else { "tray" },
        "playing": playing,
        "games": games,
        "launchers": launchers,
        "os": os,
        "events": events.iter().map(|(k, v)| (k.to_string(), serde_json::Value::from(*v))).collect::<serde_json::Map<String, serde_json::Value>>(),
    })
}

/// Makes sure this install has an id, creating and saving one the first time.
pub fn install_id(state: &Arc<AppState>) -> String {
    let existing = state.settings().install_id;
    if !existing.is_empty() {
        return existing;
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    {
        let mut inner = state.inner.lock();
        inner.settings.install_id = id.clone();
    }
    state.persist_settings();
    id
}

fn gather(app: &AppHandle, state: &Arc<AppState>) -> serde_json::Value {
    let id = install_id(state);
    let version = app.package_info().version.to_string();
    let on_screen = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
        .unwrap_or(false);
    let playing = state.current_game().map(|(_, name)| name);
    let (language, games, launchers) = {
        let inner = state.inner.lock();
        let installed: Vec<_> = inner.library.iter().filter(|g| g.installed).collect();
        let mut launchers: Vec<String> = installed.iter().map(|g| format!("{:?}", g.source).to_lowercase()).collect();
        launchers.sort();
        launchers.dedup();
        (inner.settings.language.clone(), installed.len(), launchers)
    };
    let os = sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_string());
    let events = take_counts();
    let body = payload(&id, &version, &language, on_screen, playing.as_deref(), games, &launchers, &os, &events);
    // If the send fails the caller puts the counts back; hand them along.
    serde_json::json!({ "body": body, "events": events.iter().map(|(k, v)| serde_json::json!([k, v])).collect::<Vec<_>>() })
}

fn send_once(app: &AppHandle, state: &Arc<AppState>) {
    let url = std::env::var("GAMEHUB_STATS_URL").unwrap_or_else(|_| STATS_URL.to_string());
    let gathered = gather(app, state);
    let body = gathered["body"].clone();
    let events: Vec<(&'static str, u32)> = gathered["events"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|pair| {
                    let name = pair[0].as_str()?;
                    let n = pair[1].as_u64()? as u32;
                    EVENTS.iter().find(|e| **e == name).map(|e| (*e, n))
                })
                .collect()
        })
        .unwrap_or_default();

    let ok = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(format!("GameHub/{}", app.package_info().version))
        .build()
        .and_then(|client| {
            client
                .post(&url)
                .header("content-type", "application/json")
                .body(body.to_string())
                .send()
        })
        .map(|response| response.status().is_success())
        .unwrap_or(false);
    if !ok {
        restore_counts(&events);
    }
}

/// Starts the reporter: one message now, then one every five minutes for as
/// long as the app runs. Off the UI thread, so a slow network costs nothing.
pub fn start(app: AppHandle, state: Arc<AppState>) {
    std::thread::Builder::new()
        .name("gamehub-stats".into())
        .spawn(move || loop {
            send_once(&app, &state);
            std::thread::sleep(INTERVAL);
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_carries_only_anonymous_facts() {
        let body = payload(
            "abc123",
            "1.9.0",
            "nb",
            true,
            Some("Rocket League"),
            12,
            &["steam".into(), "epic".into()],
            "Windows 11 (26100)",
            &[("screenshot", 3), ("clip", 1)],
        );
        assert_eq!(body["id"], "abc123");
        assert_eq!(body["state"], "open");
        assert_eq!(body["playing"], "Rocket League");
        assert_eq!(body["games"], 12);
        assert_eq!(body["launchers"][1], "epic");
        assert_eq!(body["events"]["screenshot"], 3);
        assert!(body.get("path").is_none() && body.get("user").is_none());
        let text = body.to_string();
        assert!(!text.contains("Users\\"), "no file paths leave the machine");
    }

    #[test]
    fn counts_are_taken_once_and_come_back_when_undelivered() {
        hit("freeze");
        hit("freeze");
        hit("not-a-feature");
        let taken = take_counts();
        assert!(taken.contains(&("freeze", 2)));
        assert!(take_counts().is_empty(), "taking empties the counters");
        restore_counts(&taken);
        assert!(take_counts().contains(&("freeze", 2)), "a failed send keeps the counts");
    }

    #[test]
    fn a_tray_app_says_so() {
        let body = payload("x", "1", "en", false, None, 0, &[], "Windows", &[]);
        assert_eq!(body["state"], "tray");
        assert!(body["playing"].is_null());
        assert_eq!(body["events"].as_object().map(|m| m.len()), Some(0));
    }
}
