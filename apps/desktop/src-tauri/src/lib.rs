//! GameHub — the Tauri application.
//!
//! The Rust side owns everything that touches the machine: reading launcher
//! data, starting games, tracking play activity. The web view owns the
//! interface and nothing else. That split is deliberate — see
//! `capabilities/default.json`, which grants no filesystem, shell or HTTP
//! permission to the front end at all.

mod artwork;
mod assets;
mod audio;
mod freeze;
mod capture;
mod hotkeys;
mod quicktools;
mod replay;
mod autostart;
mod commands;
mod launch;
mod msg;
mod overlay;
mod telemetry;
mod tray;
mod wallpaper;
mod perf;
mod process;
mod state;
mod store;
mod watcher;

use std::sync::Arc;

use tauri::{Emitter, Manager, WindowEvent};

use state::AppState;

pub fn run() {
    tauri::Builder::default()
        // One instance only: a second launch focuses the running window rather
        // than starting a second scanner over the same files.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            commands::open_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // Required by relaunch() after an update installs.
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;

            // Before anything is read or written: if this is the first launch
            // after an update, copy the previous version's data aside. Nothing
            // the new build does can then cost the user their library.
            // package_info().version comes from tauri.conf.json, not Cargo.toml
            // — which matters, because the version in tauri.conf.json is the one
            // the release workflow reads and the only one anybody edits.
            let app_version = app.package_info().version.to_string();

            let paths = crate::store::Paths::new(data_dir.clone());
            if let Some(snapshot) = paths.snapshot_if_updated(&app_version) {
                println!(
                    "backed up {} file(s) before running {app_version} for the first time: {}",
                    snapshot.files.len(),
                    snapshot.id
                );
            }
            drop(paths);

            let state = AppState::new(data_dir);
            app.manage(state.clone());

            // Let the web view read the covers, screenshots, clips and
            // background it is about to be asked to display.
            crate::assets::grant(app.handle(), &state);

            tray::build(app.handle(), &state.settings().language)?;
            overlay::create(app.handle());

            // The updater plugin, so the front end's `check()` and
            // `downloadAndInstall()` have something to talk to. Registered
            // rather than `?`-ed: a build with no endpoint configured should
            // still run perfectly well as an offline app.
            // Registration fails when tauri.conf.json has no updater section —
            // which is the normal case for a build made on your own PC, since
            // signing an update needs a key. The app is fully functional either
            // way; it just will not offer to update itself.
            match app.handle().plugin(tauri_plugin_updater::Builder::new().build()) {
                Ok(()) => crate::state::set_updates_available(true),
                Err(error) => {
                    crate::state::set_updates_available(false);
                    println!("this build does not update itself ({error})");
                }
            }

            // Started by the Run key: come up in the tray, not on screen.
            if std::env::args().any(|a| a == "--tray") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // The first scan runs off the UI thread so the window paints
            // immediately, even on a machine with nine launchers installed.
            let handle = app.handle().clone();
            let scan_state = state.clone();
            std::thread::spawn(move || {
                commands::run_scan(&handle, &scan_state);
            });

            // Global shortcuts and the clipboard watcher, both driven by the
            // same settings the Shortcut Center edits.
            hotkeys::reregister(app.handle(), state.clone());
            hotkeys::spawn_clipboard_poll(app.handle().clone(), state.clone());

            // Anonymous usage statistics for the website's admin page.
            telemetry::start(app.handle().clone(), state.clone());

            // A freeze point from before a restart holds processes that no
            // longer exist; say so rather than offering to resume them.
            {
                let mut inner = state.inner.lock();
                let changed = gamehub_detect::freeze::reconcile(&mut inner.freezes, crate::freeze::is_alive);
                drop(inner);
                if changed > 0 {
                    state.persist_freezes();
                }
            }

            // 1.5.0 shipped with "active only" on; the default is now off. Apply
            // the new default once, unless the user has set it themselves.
            {
                let mut inner = state.inner.lock();
                if !inner.settings.active_only_chosen {
                    inner.settings.track_active_only = false;
                    inner.settings.active_only_chosen = true;
                    drop(inner);
                    state.persist_settings();
                }
            }

            // Sessions logged by versions that counted wall time can be a game
            // left open for a day; those are not play and are dropped once.
            {
                let mut inner = state.inner.lock();
                let dropped = gamehub_detect::activity::prune_impossible(&mut inner.activity);
                drop(inner);
                if dropped > 0 {
                    println!("dropped {dropped} impossible session(s) from before active tracking");
                    state.persist_activity();
                }
            }

            // Arm the replay buffer if it was left on. Failure here is
            // reported, never fatal: GameHub is a library first.
            if state.settings().replay.enabled {
                if let Err(error) = commands::apply_replay(app.handle(), &state) {
                    eprintln!("replay could not start: {error}");
                    let _ = app.handle().emit("toast", (msg::plain("toast_replay_not_started"), error));
                }
            }

            watcher::spawn(app.handle().clone(), state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.app_handle().state::<Arc<AppState>>();
                if state.settings().minimise_to_tray {
                    // Closing hides; quitting is done from the tray menu. That
                    // is what keeps new-game detection running.
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_library,
            commands::get_running_games,
            commands::scan_now,
            commands::launch_game,
            commands::set_favorite,
            commands::set_hidden,
            commands::rename_game,
            commands::set_tags,
            commands::open_game_folder,
            commands::get_settings,
            commands::save_settings,
            commands::add_game_folder,
            commands::add_manual_game,
            commands::remove_game,
            commands::restore_hidden,
            commands::hidden_count,
            commands::refresh_artwork,
            commands::read_image_for_crop,
            commands::set_custom_cover,
            commands::clear_custom_cover,
            commands::get_activity,
            commands::get_quests,
            commands::get_calendar_month,
            commands::clear_activity,
            commands::take_screenshot,
            commands::get_screenshots,
            commands::set_screenshot_favorite,
            commands::delete_screenshot,
            commands::screenshot_folder,
            commands::list_backups,
            commands::get_recoveries,
            commands::backup_now,
            commands::restore_backup,
            commands::data_folder,
            commands::updates_supported,
            commands::replay_audio_devices,
            commands::list_displays,
            commands::get_clipboard,
            commands::pin_clip,
            commands::delete_clip,
            commands::clear_clipboard,
            commands::sample_performance,
            commands::get_shortcuts,
            commands::set_shortcut,
            commands::reset_shortcut,
            commands::replay_status,
            commands::set_replay_enabled,
            commands::save_replay,
            commands::get_clips,
            commands::delete_replay_clip,
            commands::set_clip_favorite,
            commands::freeze_status,
            commands::freeze_now,
            commands::resume_freeze,
            commands::restore_freeze_save,
            commands::delete_freeze,
            commands::get_freezes,
            commands::find_save_folders,
            commands::set_save_folder,
            commands::get_save_folder,
            commands::set_freeze_note,
            commands::trim_clip,
            commands::wallpaper_status,
            commands::set_wallpaper,
            commands::forget_wallpaper,
            commands::default_wallpaper,
            commands::startup_greeting,
        ])
        .run(tauri::generate_context!())
        .expect("GameHub failed to start");
}
