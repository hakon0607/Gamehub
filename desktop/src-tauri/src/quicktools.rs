//! The Quick Tools overlay.
//!
//! A second, small, always-on-top window that appears over whatever is in
//! front. It is created once and then shown and hidden, because creating a
//! webview takes long enough to feel slow on a shortcut.

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "quicktools";

pub fn toggle(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return;
    }

    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html?quicktools=1".into()))
        .title("Quick Tools")
        .inner_size(560.0, 420.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .center()
        .build();

    match built {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(error) => eprintln!("Quick Tools could not open: {error}"),
    }
}
