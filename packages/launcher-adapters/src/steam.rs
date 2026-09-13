//! Steam.
//!
//! Steam is the well-behaved one: it writes its install path to the registry,
//! lists every library folder in `steamapps/libraryfolders.vdf`, and drops one
//! `appmanifest_<appid>.acf` per installed game. Reading those three things is
//! both complete and stable, so GameHub never has to guess or scan drives.
//!
//! Games are started with `steam://rungameid/<appid>`, which is Steam's own
//! documented handler. GameHub never touches the game executable — that would
//! break the overlay, cloud saves and anything with DRM.

use std::path::{Path, PathBuf};

use gamehub_detect::{
    env::Hive,
    model::{Game, GameSource, LaunchMethod},
    vdf, Env,
};

use crate::{first_existing, AdapterScan, LauncherAdapter};

pub struct SteamAdapter;

/// Entries that live in a Steam library but are not games. 228980 is the
/// Steamworks redistributable package, which is installed alongside almost
/// everything and would otherwise appear in every user's library.
const NON_GAME_APPIDS: &[&str] = &["228980", "1070560", "1391110", "1493710", "1826330", "2180100"];

/// StateFlags is a bitfield; bit 2 (value 4) means "fully installed".
const STATE_FULLY_INSTALLED: u64 = 4;

impl LauncherAdapter for SteamAdapter {
    fn source(&self) -> GameSource {
        GameSource::Steam
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let Some(root) = find_steam(env) else {
            return AdapterScan::missing(GameSource::Steam);
        };

        let libraries = library_folders(&root);
        let mut games = Vec::new();
        for library in &libraries {
            games.extend(games_in_library(library));
        }
        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        games.dedup_by(|a, b| a.id == b.id);

        let scan = AdapterScan::found(GameSource::Steam, Some(root.to_string_lossy().into_owned()), games);
        if libraries.len() <= 1 {
            scan
        } else {
            scan
        }
    }
}

/// The install path, from the registry first and a default second.
pub fn find_steam(env: &Env) -> Option<PathBuf> {
    let from_registry = env
        .reg_string(Hive::CurrentUser, "Software\\Valve\\Steam", "SteamPath")
        .or_else(|| env.reg_string_both_views(Hive::LocalMachine, "SOFTWARE\\Valve\\Steam", "InstallPath"))
        .map(PathBuf::from);

    if let Some(path) = from_registry.filter(|p| p.exists()) {
        return Some(path);
    }

    first_existing([
        env.folders.program_files_x86.join("Steam"),
        env.folders.program_files.join("Steam"),
    ])
}

/// Every library folder Steam knows about, including the one inside the install
/// directory. Reading this file is what makes games on a second drive work.
pub fn library_folders(steam_root: &Path) -> Vec<PathBuf> {
    let mut folders = vec![steam_root.to_path_buf()];

    let manifest = first_existing([
        steam_root.join("steamapps").join("libraryfolders.vdf"),
        // Steam wrote it here before 2021, and plenty of installs still have it.
        steam_root.join("config").join("libraryfolders.vdf"),
    ]);

    if let Some(manifest) = manifest {
        if let Ok(text) = std::fs::read_to_string(&manifest) {
            if let Ok(parsed) = vdf::parse(&text) {
                if let Some(root) = parsed.get("libraryfolders").and_then(|v| v.as_object()) {
                    for (_, entry) in root {
                        // Modern format: an object with a "path". Ancient format:
                        // the value is the path itself, keyed by an index.
                        let path = match entry {
                            vdf::Value::Object(_) => entry.get_str("path").map(str::to_string),
                            vdf::Value::String(s) => Some(s.clone()),
                        };
                        if let Some(path) = path {
                            let path = PathBuf::from(path.replace("\\\\", "\\"));
                            if path.exists() && !folders.contains(&path) {
                                folders.push(path);
                            }
                        }
                    }
                }
            }
        }
    }

    folders
}

fn games_in_library(library: &Path) -> Vec<Game> {
    let steamapps = library.join("steamapps");
    let Ok(entries) = std::fs::read_dir(&steamapps) else {
        return Vec::new();
    };

    let mut games = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
            continue;
        }
        if let Some(game) = read_manifest(&entry.path(), &steamapps) {
            games.push(game);
        }
    }
    games
}

/// Turns one `appmanifest_*.acf` into a game, or `None` if it is not a game the
/// user would expect to see.
fn read_manifest(path: &Path, steamapps: &Path) -> Option<Game> {
    let text = std::fs::read_to_string(path).ok()?;
    let parsed = vdf::parse(&text).ok()?;
    let state = parsed.get("AppState")?;

    let appid = state.get_str("appid")?.trim().to_string();
    if NON_GAME_APPIDS.contains(&appid.as_str()) {
        return None;
    }

    let name = state.get_str("name").unwrap_or("").trim();
    if name.is_empty() {
        return None;
    }

    let flags = state.get_u64("StateFlags").unwrap_or(0);
    let fully_installed = flags & STATE_FULLY_INSTALLED != 0;

    let mut game = Game::new(GameSource::Steam, &appid, name);
    game.installed = fully_installed;

    if let Some(dir) = state.get_str("installdir") {
        let install = steamapps.join("common").join(dir);
        // A manifest can outlive the files it describes — Steam leaves one
        // behind if a drive is disconnected mid-uninstall.
        if install.exists() {
            game.install_dir = Some(install.to_string_lossy().into_owned());
        } else {
            game.installed = false;
        }
    }

    game.size_bytes = state.get_u64("SizeOnDisk").filter(|&s| s > 0);
    if let Some(seconds) = state.get_u64("LastPlayed").filter(|&s| s > 0) {
        game.last_played = gamehub_detect::unix_to_iso8601(seconds as i64);
    }

    // rungameid is the id that works for every kind of Steam entry, including
    // shortcuts and games with multiple launch options.
    game.launch = Some(LaunchMethod::Uri {
        uri: format!("steam://rungameid/{appid}"),
    });

    Some(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::FakeRegistry;
    use std::fs;

    /// Builds a Steam install with two libraries, the way a real machine looks.
    fn fixture() -> (tempfile::TempDir, Env) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let steam = root.join("Program Files (x86)/Steam");
        let second = root.join("D_SteamLibrary");

        for (library, apps) in [
            (&steam, vec![("440", "Team Fortress 2", "Team Fortress 2", 4u64, 24_696_061_952u64, 1_735_689_600u64)]),
            (&second, vec![("1174180", "Red Dead Redemption 2", "Red Dead Redemption 2", 4, 120_000_000_000, 0)]),
        ] {
            fs::create_dir_all(library.join("steamapps/common")).unwrap();
            for (appid, name, installdir, flags, size, last) in apps {
                fs::create_dir_all(library.join("steamapps/common").join(installdir)).unwrap();
                fs::write(
                    library.join("steamapps").join(format!("appmanifest_{appid}.acf")),
                    format!(
                        "\"AppState\"\n{{\n\t\"appid\" \"{appid}\"\n\t\"name\" \"{name}\"\n\t\"StateFlags\" \"{flags}\"\n\t\"installdir\" \"{installdir}\"\n\t\"SizeOnDisk\" \"{size}\"\n\t\"LastPlayed\" \"{last}\"\n}}\n"
                    ),
                )
                .unwrap();
            }
        }

        // The redistributable package that is installed on nearly every machine.
        fs::write(
            steam.join("steamapps/appmanifest_228980.acf"),
            "\"AppState\"\n{\n\t\"appid\" \"228980\"\n\t\"name\" \"Steamworks Common Redistributables\"\n\t\"StateFlags\" \"4\"\n\t\"installdir\" \"Steamworks Shared\"\n}\n",
        )
        .unwrap();

        fs::write(
            steam.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\" \"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\" \"{}\"\n\t}}\n}}\n",
                steam.to_string_lossy().replace('\\', "\\\\"),
                second.to_string_lossy().replace('\\', "\\\\"),
            ),
        )
        .unwrap();

        let registry = FakeRegistry::new().with(
            Hive::CurrentUser,
            "Software\\Valve\\Steam",
            "SteamPath",
            &steam.to_string_lossy(),
        );
        let env = Env::fixture(root, registry);
        (dir, env)
    }

    #[test]
    fn finds_games_across_every_library_folder() {
        let (_dir, env) = fixture();
        let scan = SteamAdapter.scan(&env);
        assert!(scan.status.detected);
        let names: Vec<&str> = scan.games.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, vec!["Red Dead Redemption 2", "Team Fortress 2"]);
    }

    #[test]
    fn skips_the_redistributable_package() {
        let (_dir, env) = fixture();
        let scan = SteamAdapter.scan(&env);
        assert!(!scan.games.iter().any(|g| g.name.contains("Redistributables")));
    }

    #[test]
    fn reads_size_last_played_and_install_dir() {
        let (_dir, env) = fixture();
        let scan = SteamAdapter.scan(&env);
        let tf2 = scan.games.iter().find(|g| g.source_id == "440").unwrap();
        assert_eq!(tf2.size_bytes, Some(24_696_061_952));
        assert_eq!(tf2.last_played.as_deref(), Some("2025-01-01T00:00:00Z"));
        assert!(tf2.install_dir.as_ref().unwrap().ends_with("Team Fortress 2"));
    }

    #[test]
    fn launches_through_steams_own_handler_never_an_executable() {
        let (_dir, env) = fixture();
        let scan = SteamAdapter.scan(&env);
        for game in &scan.games {
            match game.launch.as_ref().unwrap() {
                LaunchMethod::Uri { uri } => assert!(uri.starts_with("steam://rungameid/")),
                other => panic!("Steam games must launch through Steam, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_manifest_whose_files_are_gone_is_reported_as_not_installed() {
        let (dir, env) = fixture();
        fs::remove_dir_all(dir.path().join("Program Files (x86)/Steam/steamapps/common/Team Fortress 2")).unwrap();
        let scan = SteamAdapter.scan(&env);
        let tf2 = scan.games.iter().find(|g| g.source_id == "440").unwrap();
        assert!(!tf2.installed);
    }

    #[test]
    fn a_downloading_game_is_listed_but_not_marked_installed() {
        let (dir, env) = fixture();
        // StateFlags 1026 = update running, not fully installed.
        fs::write(
            dir.path().join("Program Files (x86)/Steam/steamapps/appmanifest_570.acf"),
            "\"AppState\"\n{\n\t\"appid\" \"570\"\n\t\"name\" \"Dota 2\"\n\t\"StateFlags\" \"1026\"\n\t\"installdir\" \"dota 2 beta\"\n}\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("Program Files (x86)/Steam/steamapps/common/dota 2 beta")).unwrap();
        let scan = SteamAdapter.scan(&env);
        let dota = scan.games.iter().find(|g| g.source_id == "570").unwrap();
        assert!(!dota.installed);
    }

    #[test]
    fn a_corrupt_manifest_does_not_take_down_the_scan() {
        let (dir, env) = fixture();
        fs::write(
            dir.path().join("Program Files (x86)/Steam/steamapps/appmanifest_999.acf"),
            "\"AppState\" {\n \"appid\" \"999\"\n",
        )
        .unwrap();
        let scan = SteamAdapter.scan(&env);
        assert_eq!(scan.games.len(), 2, "the two good manifests still come through");
    }

    #[test]
    fn steam_missing_is_a_normal_result_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::fixture(dir.path(), FakeRegistry::new());
        let scan = SteamAdapter.scan(&env);
        assert!(!scan.status.detected);
        assert!(scan.games.is_empty());
    }
}
