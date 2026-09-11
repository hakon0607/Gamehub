//! The tray icon and its menu.
//!
//! The menu is the one piece of interface the Rust side draws itself, so it is
//! the one place the Rust side needs words. The three labels exist in every
//! language the web view offers (`src/i18n`), and the menu is rebuilt when the
//! language setting changes so it never lags behind the rest of the app.

use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::state::AppState;

/// `(open GameHub, look for new games, quit)` in the given language.
pub fn labels(language: &str) -> (&'static str, &'static str, &'static str) {
    match language {
        "nb" => ("Åpne GameHub", "Se etter nye spill", "Avslutt"),
        "sv" => ("Öppna GameHub", "Leta efter nya spel", "Avsluta"),
        "da" => ("Åbn GameHub", "Se efter nye spil", "Afslut"),
        "fi" => ("Avaa GameHub", "Etsi uusia pelejä", "Lopeta"),
        "de" => ("GameHub öffnen", "Nach neuen Spielen suchen", "Beenden"),
        "fr" => ("Ouvrir GameHub", "Chercher de nouveaux jeux", "Quitter"),
        "es" => ("Abrir GameHub", "Buscar juegos nuevos", "Salir"),
        "pl" => ("Otwórz GameHub", "Szukaj nowych gier", "Zakończ"),
        "nl" => ("GameHub openen", "Zoeken naar nieuwe games", "Afsluiten"),
        _ => ("Open GameHub", "Look for new games", "Quit"),
    }
}

/// The languages `labels` knows, so a test can check the list against the
/// web view's.
#[cfg(test)]
pub const LANGUAGES: [&str; 10] = ["en", "nb", "sv", "da", "fi", "de", "fr", "es", "pl", "nl"];

fn menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (open, scan, quit) = labels(language);
    let open = MenuItem::with_id(app, "open", open, true, None::<&str>)?;
    let scan = MenuItem::with_id(app, "scan", scan, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&open, &scan, &quit])
}

pub fn build(app: &AppHandle, language: &str) -> tauri::Result<()> {
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("the tray needs the app icon".into())
        })?)
        .tooltip("GameHub")
        .menu(&menu(app, language)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "scan" => {
                let handle = app.clone();
                let state = app.state::<Arc<AppState>>().inner().clone();
                std::thread::spawn(move || {
                    crate::commands::run_scan(&handle, &state);
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// Swaps the menu for one in `language`. Called when the setting changes.
pub fn relabel(app: &AppHandle, language: &str) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = menu(app, language) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_its_own_labels() {
        let english = labels("en");
        for language in LANGUAGES.iter().filter(|l| **l != "en") {
            assert_ne!(labels(language), english, "{language} falls back to English");
        }
    }

    #[test]
    fn unknown_languages_get_english() {
        assert_eq!(labels("xx"), labels("en"));
    }
}
