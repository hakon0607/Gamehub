//! The scan: run every adapter, fold the result into the stored library, and
//! report what changed.
//!
//! This is what the onboarding screen, the background timer and the manual
//! "Rescan" button all call. One adapter failing or being slow must never stop
//! the others, and a scan must never block on the network — metadata is fetched
//! afterwards, separately.

use std::path::PathBuf;
use std::time::Instant;

use gamehub_detect::{
    library,
    model::{Game, LauncherStatus, ScanResult},
    Env,
};

use crate::local::LocalAdapter;

pub struct ScanOptions {
    /// Folders the user added by hand, scanned by the local adapter.
    pub extra_folders: Vec<PathBuf>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self { extra_folders: Vec::new() }
    }
}

/// Runs every adapter against `env` and merges the result into `stored`.
pub fn scan_into(stored: &mut Vec<Game>, env: &Env, options: &ScanOptions) -> ScanResult {
    let started = Instant::now();

    let mut adapters = crate::all_adapters();
    adapters.push(Box::new(LocalAdapter {
        folders: options.extra_folders.clone(),
    }));

    let mut launchers: Vec<LauncherStatus> = Vec::new();
    let mut found: Vec<Game> = Vec::new();

    for adapter in adapters {
        // An adapter is not allowed to take the scan down. `scan` returns
        // rather than throwing, but a panic in one launcher's parsing should
        // still leave the other launchers working.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| adapter.scan(env)));
        match result {
            Ok(scan) => {
                launchers.push(scan.status);
                found.extend(scan.games);
            }
            Err(_) => launchers.push(LauncherStatus {
                source: adapter.source(),
                detected: false,
                install_dir: None,
                game_count: 0,
                limitation: Some(
                    "This launcher could not be read on this machine. Everything else was scanned \
                     normally; please report it so the adapter can be fixed."
                        .into(),
                ),
            }),
        }
    }

    let outcome = library::merge(stored, found);

    ScanResult {
        launchers,
        games: stored.clone(),
        new_game_ids: outcome.new_ids,
        scanned_at: gamehub_detect::now_iso8601(),
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

/// The directories worth putting a filesystem watcher on: the handful of places
/// that change when a game is installed. Watching these costs nothing while
/// idle, which is the point — GameHub must never sit and re-read whole drives.
pub fn watch_targets(env: &Env, extra_folders: &[PathBuf]) -> Vec<PathBuf> {
    let mut targets: Vec<PathBuf> = Vec::new();

    if let Some(steam) = crate::steam::find_steam(env) {
        for library in crate::steam::library_folders(&steam) {
            targets.push(library.join("steamapps"));
        }
    }
    targets.push(crate::epic::manifest_dir(env));
    targets.push(env.folders.program_data.join("Riot Games/Metadata"));
    targets.extend(extra_folders.iter().cloned());

    targets.retain(|p| p.is_dir());
    targets.sort();
    targets.dedup();
    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::{env::Hive, FakeRegistry};
    use std::fs;

    fn steam_fixture(tmp: &std::path::Path, apps: &[(&str, &str)]) -> Env {
        let steam = tmp.join("Program Files (x86)/Steam");
        fs::create_dir_all(steam.join("steamapps/common")).unwrap();
        for (appid, name) in apps {
            fs::create_dir_all(steam.join("steamapps/common").join(name)).unwrap();
            fs::write(
                steam.join("steamapps").join(format!("appmanifest_{appid}.acf")),
                format!("\"AppState\"\n{{\n\"appid\" \"{appid}\"\n\"name\" \"{name}\"\n\"StateFlags\" \"4\"\n\"installdir\" \"{name}\"\n}}\n"),
            )
            .unwrap();
        }
        Env::fixture(
            tmp,
            FakeRegistry::new().with(Hive::CurrentUser, "Software\\Valve\\Steam", "SteamPath", &steam.to_string_lossy()),
        )
    }

    #[test]
    fn a_scan_reports_every_launcher_including_the_missing_ones() {
        let tmp = tempfile::tempdir().unwrap();
        let env = steam_fixture(tmp.path(), &[("440", "Team Fortress 2")]);
        let mut library = Vec::new();
        let result = scan_into(&mut library, &env, &ScanOptions::default());

        assert_eq!(result.games.len(), 1);
        assert!(result.launchers.iter().any(|l| l.source.slug() == "steam" && l.detected));
        assert!(
            result.launchers.iter().any(|l| l.source.slug() == "epic" && !l.detected),
            "a launcher that is not installed still gets a row, so onboarding can show it"
        );
    }

    #[test]
    fn installing_a_new_game_between_scans_is_reported_once() {
        let tmp = tempfile::tempdir().unwrap();
        let mut library = Vec::new();
        let env = steam_fixture(tmp.path(), &[("440", "Team Fortress 2")]);
        scan_into(&mut library, &env, &ScanOptions::default());

        let env = steam_fixture(tmp.path(), &[("440", "Team Fortress 2"), ("1174180", "Red Dead Redemption 2")]);
        let second = scan_into(&mut library, &env, &ScanOptions::default());
        assert_eq!(second.new_game_ids, vec!["steam:1174180".to_string()]);

        let third = scan_into(&mut library, &env, &ScanOptions::default());
        assert!(third.new_game_ids.is_empty(), "the same game must not be announced twice");
    }

    #[test]
    fn watch_targets_are_the_folders_that_change_on_install() {
        let tmp = tempfile::tempdir().unwrap();
        let env = steam_fixture(tmp.path(), &[("440", "Team Fortress 2")]);
        let targets = watch_targets(&env, &[]);
        assert!(targets.iter().any(|p| p.ends_with("steamapps")));
        assert!(
            targets.iter().all(|p| p.is_dir()),
            "watching a folder that does not exist would fail at startup"
        );
    }

    #[test]
    fn extra_folders_are_scanned_and_watched() {
        let tmp = tempfile::tempdir().unwrap();
        let extra = tmp.path().join("MyGames");
        fs::create_dir_all(extra.join("Celeste")).unwrap();
        fs::write(extra.join("Celeste/Celeste.exe"), b"MZ").unwrap();

        let env = steam_fixture(tmp.path(), &[]);
        let options = ScanOptions { extra_folders: vec![extra.clone()] };
        let mut library = Vec::new();
        let result = scan_into(&mut library, &env, &options);

        assert!(result.games.iter().any(|g| g.name == "Celeste"));
        assert!(watch_targets(&env, &options.extra_folders).contains(&extra));
    }
}
