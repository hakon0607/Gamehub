//! Global shortcuts, and the clipboard poller.
//!
//! Global shortcuts are the ones that have to work while a game has focus, so
//! they are registered with Windows through Tauri's plugin. Whenever the user
//! rebinds one, everything is unregistered and registered again from the single
//! `Bindings` registry — there is no second list to drift out of sync.

use std::sync::Arc;
use std::time::Duration;

use gamehub_detect::shortcuts::Action;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::state::AppState;

/// Registers every global shortcut, replacing whatever was registered before.
pub fn reregister(app: &AppHandle, state: Arc<AppState>) {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    for (action, binding) in state.inner.lock().bindings.global() {
        // A combination Windows will not give us — because another program
        // already holds it — is skipped with a note rather than taking the
        // others down with it.
        if let Err(error) = manager.on_shortcut(binding.as_str(), {
            let app = app.clone();
            move |_, _, event| {
                if event.state() == ShortcutState::Pressed {
                    handle(&app, action);
                }
            }
        }) {
            eprintln!("could not register {binding} for {}: {error}", action.id());
            let _ = app.emit("shortcut-unavailable", (action.id(), binding.clone()));
        }
    }
}

fn handle(app: &AppHandle, action: Action) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    match action {
        Action::OpenGameHub => show_main(app),
        Action::QuickTools => crate::quicktools::toggle(app),
        Action::Screenshot => {
            let app = app.clone();
            std::thread::spawn(move || {
                let state = app.state::<Arc<AppState>>().inner().clone();
                match crate::commands::capture_screenshot(&app, &state) {
                    Ok(shot) => {
                        let _ = app.emit("toast", ("Screenshot lagret", shot.game_name));
                    }
                    Err(error) => {
                        let _ = app.emit("toast", ("Screenshot mislyktes", error));
                    }
                }
            });
        }
        Action::SaveReplay => {
            let app = app.clone();
            std::thread::spawn(move || {
                let state = app.state::<Arc<AppState>>().inner().clone();
                match crate::commands::save_replay_now(&app, &state, None) {
                    Ok(clip) => {
                        let _ = app.emit("toast", ("Klipp lagret", clip.game_name));
                    }
                    Err(error) => {
                        let _ = app.emit("toast", ("Klippet ble ikke lagret", error));
                    }
                }
            });
        }
        Action::ToggleReplay => {
            let app = app.clone();
            std::thread::spawn(move || {
                let state = app.state::<Arc<AppState>>().inner().clone();
                match crate::commands::toggle_replay(&app, &state) {
                    Ok(on) => {
                        let _ = app.emit(
                            "toast",
                            (
                                if on { "Replay er på" } else { "Replay er av" },
                                if on {
                                    "Alt eldre enn bufferet slettes fortløpende."
                                } else {
                                    "Ingenting tas opp nå."
                                },
                            ),
                        );
                    }
                    Err(error) => {
                        let _ = app.emit("toast", ("Replay kunne ikke byttes", error));
                    }
                }
            });
        }
        Action::FreezeGame => {
            let app = app.clone();
            std::thread::spawn(move || {
                let state = app.state::<Arc<AppState>>().inner().clone();
                match crate::commands::toggle_freeze(&app, &state) {
                    Ok((frozen, name)) => {
                        let _ = app.emit(
                            "toast",
                            (
                                if frozen { "Spillet er frosset" } else { "Spillet fortsetter" },
                                if frozen {
                                    format!("{name} står stille. Trykk samme tast for å fortsette.")
                                } else {
                                    name
                                },
                            ),
                        );
                    }
                    Err(error) => {
                        let _ = app.emit("toast", ("Kunne ikke fryse", error));
                    }
                }
            });
        }
        Action::ToggleOverlay => {
            let _ = app.emit("toggle-overlay", ());
            show_main(app);
        }
        _ => {
            // App-scope actions are handled by the window; reaching here means
            // one was registered globally by mistake.
            let _ = state;
        }
    }
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Watches the clipboard.
///
/// Polling is the only way to see *other* programs' copies; there is no change
/// event Windows offers a sandboxed app. One second is frequent enough to feel
/// instant and far too rare to measure.
pub fn spawn_clipboard_poll(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        use tauri_plugin_clipboard_manager::ClipboardExt;
        let mut previous = String::new();

        loop {
            std::thread::sleep(Duration::from_secs(1));
            if !state.inner.lock().clipboard.enabled {
                continue;
            }
            let Ok(text) = app.clipboard().read_text() else { continue };
            if text == previous || text.trim().is_empty() {
                continue;
            }
            previous = text.clone();

            let stored = {
                let mut inner = state.inner.lock();
                let now = gamehub_detect::now_iso8601();
                gamehub_detect::clipboard::record(&mut inner.clipboard, &text, &now)
            };
            if stored {
                state.persist_clipboard();
                let _ = app.emit("clipboard-updated", ());
            }
        }
    });
}
