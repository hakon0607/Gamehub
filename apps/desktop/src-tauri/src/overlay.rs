//! The little popup over the game.
//!
//! When a shortcut does something while GameHub is hidden — a screenshot, a
//! clip, a freeze — the main window's toast is invisible. This is a second,
//! tiny, always-on-top window in the bottom-right corner that shows the same
//! message for a few seconds and then hides again. It never takes focus (the
//! game keeps the keyboard) and ignores the mouse (a click goes through to
//! whatever is under it).
//!
//! The honest limit: a game in *exclusive* fullscreen draws straight to the
//! screen and covers every window, this one included. Borderless windowed —
//! the default in most current games — shows it fine. Settings say so.
//!
//! The window is created once at startup, hidden, because a webview takes
//! long enough to build that a popup made on demand would arrive late.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "notify";

/// Logical size of the popup window. The card inside is a little smaller,
/// leaving room for its shadow.
const WIDTH: f64 = 400.0;
const HEIGHT: f64 = 104.0;
/// Distance from the screen's right and bottom edges, in logical pixels.
const MARGIN: f64 = 24.0;

/// What the popup shows. `title` and `body` are message codes or plain text
/// exactly as the main window's toast gets them; the popup translates them
/// itself, in `language`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Popup {
    pub title: String,
    pub body: String,
    /// `ok`, `error`, `frozen`, `shot`, `clip`, `replay` — picks the icon.
    pub kind: String,
    pub language: String,
    pub theme: String,
    pub seconds: f64,
}

/// Creates the hidden popup window. Called once from setup; a failure is
/// logged and costs nothing but the popup.
pub fn create(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_some() {
        return;
    }
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html?overlay=1".into()))
        .title("GameHub")
        .inner_size(WIDTH, HEIGHT)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(false)
        .visible(false)
        .build();
    match built {
        Ok(window) => {
            // Clicks go through to the game underneath.
            let _ = window.set_ignore_cursor_events(true);
        }
        Err(error) => eprintln!("the popup window could not be created: {error}"),
    }
}

/// Tells the user something: the main window gets its toast as always, and
/// when that window is not the one in front, the popup shows it too.
pub fn notify(app: &AppHandle, title: impl Into<String>, body: impl Into<String>) {
    let title = title.into();
    let body = body.into();
    let _ = app.emit_to("main", "toast", (title.clone(), body.clone()));

    let main_in_front = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false))
        .unwrap_or(false);
    if main_in_front {
        return;
    }

    let settings = app.state::<std::sync::Arc<crate::state::AppState>>().settings();
    if !settings.overlay_popup {
        return;
    }
    if settings.overlay_sound {
        play_sound();
    }
    show(
        app,
        Popup {
            kind: kind_of(&title).to_string(),
            title,
            body,
            language: settings.language.clone(),
            theme: settings.theme.clone(),
            seconds: 3.2,
        },
    );
}

fn show(app: &AppHandle, popup: Popup) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let (x, y) = place(size.width as f64, size.height as f64, scale);
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
    let _ = window.emit("overlay-toast", &popup);
    let _ = window.show();
}

/// Top-left corner of the popup, in physical pixels, for a screen of the
/// given physical size and scale factor: bottom-right, with a margin.
pub fn place(screen_width: f64, screen_height: f64, scale: f64) -> (i32, i32) {
    let x = screen_width - (WIDTH + MARGIN) * scale;
    let y = screen_height - (HEIGHT + MARGIN) * scale;
    (x.max(0.0).round() as i32, y.max(0.0).round() as i32)
}

/// The icon is chosen from the message code, so the Rust side needs no
/// words and the popup needs no second list of what each shortcut does.
pub fn kind_of(title_code: &str) -> &'static str {
    let code = title_code.trim_start_matches('@');
    if code.contains("failed") || code.contains("not_started") {
        "error"
    } else if code.contains("frozen") || code.contains("resumed") || code.contains("freeze") {
        "frozen"
    } else if code.contains("shot") {
        "shot"
    } else if code.contains("clip") {
        "clip"
    } else if code.contains("replay") {
        "replay"
    } else {
        "ok"
    }
}

/// A short two-note "pling", played through Windows' own PlaySound so it
/// works whether or not the popup window is allowed to autoplay audio.
pub fn play_sound() {
    static PLING: &[u8] = include_bytes!("../assets/pling.wav");
    play_wav(PLING);
}

/// The soft rising chime that goes with the logo animation when GameHub
/// opens. Same route as the pling, so it needs no audio permission.
pub fn play_startup_sound() {
    static CHIME: &[u8] = include_bytes!("../assets/startup.wav");
    play_wav(CHIME);
}

#[cfg(windows)]
fn play_wav(wav: &'static [u8]) {
    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_MEMORY: u32 = 0x0004;
    #[link(name = "winmm")]
    extern "system" {
        fn PlaySoundW(sound: *const u8, module: isize, flags: u32) -> i32;
    }
    // SAFETY: the buffer is a static, complete RIFF/WAVE file that outlives
    // the asynchronous playback, which is the one thing SND_MEMORY requires.
    unsafe {
        PlaySoundW(wav.as_ptr(), 0, SND_ASYNC | SND_NODEFAULT | SND_MEMORY);
    }
}

#[cfg(not(windows))]
fn play_wav(_wav: &'static [u8]) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_popup_sits_in_the_bottom_right_corner() {
        // 1920×1080 at 100 %: right edge minus width and margin.
        assert_eq!(place(1920.0, 1080.0, 1.0), (1920 - 424, 1080 - 128));
        // 2560×1440 at 150 %: the margin scales with the screen.
        assert_eq!(place(2560.0, 1440.0, 1.5), (2560 - 636, 1440 - 192));
    }

    #[test]
    fn a_screen_smaller_than_the_popup_still_gets_a_position() {
        assert_eq!(place(300.0, 80.0, 1.0), (0, 0));
    }

    #[test]
    fn the_icon_follows_the_message_code() {
        assert_eq!(kind_of("@toast_shot_saved"), "shot");
        assert_eq!(kind_of("@toast_shot_failed"), "error");
        assert_eq!(kind_of("@toast_clip_saved"), "clip");
        assert_eq!(kind_of("@toast_frozen"), "frozen");
        assert_eq!(kind_of("@toast_resumed"), "frozen");
        assert_eq!(kind_of("@toast_replay_on"), "replay");
        assert_eq!(kind_of("@toast_replay_not_started"), "error");
    }

    #[test]
    fn the_sound_is_a_complete_wave_file() {
        for wav in [
            include_bytes!("../assets/pling.wav") as &[u8],
            include_bytes!("../assets/startup.wav") as &[u8],
        ] {
            assert_eq!(&wav[0..4], b"RIFF");
            assert_eq!(&wav[8..12], b"WAVE");
            let declared = u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]) as usize;
            assert_eq!(declared + 8, wav.len(), "the RIFF header covers the whole file");
        }
    }
}
