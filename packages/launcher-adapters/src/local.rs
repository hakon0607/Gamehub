//! Games that belong to no launcher.
//!
//! The user points GameHub at a folder — `D:\Games`, an old backup drive, an
//! itch.io download — and each immediate subfolder is treated as one game. The
//! hard part is picking which of the executables in a game folder is the game:
//! a typical install also contains an uninstaller, a crash handler, a
//! redistributable installer and two or three tools.
//!
//! The rule used here, in order: an executable whose name resembles the folder
//! name; otherwise the largest executable that is not on the blocklist. It is a
//! heuristic, so the UI always lets the user pick a different one.

use std::path::{Path, PathBuf};

use gamehub_detect::{
    model::{Game, GameSource, LaunchMethod},
    safepath, Env,
};

use crate::{AdapterScan, LauncherAdapter};

/// Substrings that mark an executable as not-the-game. Matched case-insensitively
/// against the file stem.
const NOT_A_GAME: &[&str] = &[
    "unins", "uninstall", "setup", "installer", "redist", "vcredist", "dxsetup", "directx",
    "dotnetfx", "crashhandler", "crashreport", "crashpad", "unitycrashhandler", "ueprereqsetup",
    "launcher_helper", "config", "settings", "benchmark", "editor", "server", "dedicated",
    "vc_redist", "oalinst", "physx", "eossdk", "epicwebhelper", "cleanup", "repair",
];

/// A game folder is not scanned deeper than this. Real games keep their main
/// binary within two or three levels; going deeper only finds tooling.
const MAX_DEPTH: usize = 4;

pub struct LocalAdapter {
    pub folders: Vec<PathBuf>,
}

impl LauncherAdapter for LocalAdapter {
    fn source(&self) -> GameSource {
        GameSource::Local
    }

    fn scan(&self, _env: &Env) -> AdapterScan {
        let existing: Vec<&PathBuf> = self.folders.iter().filter(|p| p.is_dir()).collect();
        if existing.is_empty() {
            return AdapterScan::missing(GameSource::Local);
        }

        let mut games = Vec::new();
        for root in &existing {
            let Ok(entries) = std::fs::read_dir(root) else { continue };
            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                if let Some(game) = game_in_folder(&dir) {
                    games.push(game);
                }
            }
        }

        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        games.dedup_by(|a, b| a.id == b.id);
        AdapterScan::found(
            GameSource::Local,
            existing.first().map(|p| p.to_string_lossy().into_owned()),
            games,
        )
    }
}

fn game_in_folder(dir: &Path) -> Option<Game> {
    let folder_name = dir.file_name()?.to_string_lossy().into_owned();
    let executable = best_executable(dir)?;

    // The id has to survive the folder being moved between drives, so it is the
    // folder name rather than the full path.
    let mut game = Game::new(GameSource::Local, &folder_name, tidy_name(&folder_name));
    game.installed = true;
    game.install_dir = Some(dir.to_string_lossy().into_owned());
    game.size_bytes = crate::directory_size(dir, 50_000);
    game.launch = Some(LaunchMethod::Executable {
        path: executable.to_string_lossy().into_owned(),
        args: Vec::new(),
        working_dir: executable.parent().map(|p| p.to_string_lossy().into_owned()),
    });
    Some(game)
}

/// Picks the executable most likely to be the game itself.
pub fn best_executable(dir: &Path) -> Option<PathBuf> {
    let folder_key = normalise_key(&dir.file_name()?.to_string_lossy());

    let mut candidates: Vec<(PathBuf, u64, usize)> = Vec::new();
    for entry in walkdir::WalkDir::new(dir).max_depth(MAX_DEPTH).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) != Some("exe".into()) {
            continue;
        }
        let stem = path.file_stem()?.to_string_lossy().to_lowercase();
        if NOT_A_GAME.iter().any(|bad| stem.contains(bad)) {
            continue;
        }
        // Validated here as well as at launch time: a symlink pointing out of
        // the folder must never become a candidate in the first place.
        if safepath::validate_executable(path, &[dir.to_path_buf()]).is_err() {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let depth = entry.depth();
        candidates.push((path.to_path_buf(), size, depth));
    }

    // A name match beats size: "Hollow Knight.exe" in "Hollow Knight" is the
    // game even when a bundled tool is bigger.
    let named = candidates.iter().find(|(path, _, _)| {
        path.file_stem()
            .map(|s| normalise_key(&s.to_string_lossy()) == folder_key)
            .unwrap_or(false)
    });
    if let Some((path, _, _)) = named {
        return Some(path.clone());
    }

    candidates
        .into_iter()
        // Shallower first, then larger: the top-level binary is usually the game.
        .min_by(|a, b| a.2.cmp(&b.2).then(b.1.cmp(&a.1)))
        .map(|(path, _, _)| path)
}

fn normalise_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// "hollow_knight-v1.5" becomes "Hollow Knight V1.5" — readable, and the user
/// can rename it if the guess is poor.
fn tidy_name(folder: &str) -> String {
    let spaced = folder.replace(['_', '.'], " ").replace('-', " ");
    spaced
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::FakeRegistry;
    use std::fs;

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![b'M'; bytes]).unwrap();
    }

    #[test]
    fn one_game_per_subfolder_with_the_matching_executable() {
        let tmp = tempfile::tempdir().unwrap();
        let games = tmp.path().join("Games");
        write(&games.join("Hollow Knight/Hollow Knight.exe"), 100);
        write(&games.join("Hollow Knight/UnityCrashHandler64.exe"), 5000);
        write(&games.join("Hollow Knight/unins000.exe"), 9000);

        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        let scan = LocalAdapter { folders: vec![games] }.scan(&env);

        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].name, "Hollow Knight");
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Executable { path, .. } => assert!(path.ends_with("Hollow Knight.exe")),
            other => panic!("expected an executable, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_the_shallowest_largest_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let games = tmp.path().join("Games");
        write(&games.join("SomeGame/bin/tool.exe"), 100);
        write(&games.join("SomeGame/start.exe"), 4000);
        write(&games.join("SomeGame/vcredist_x64.exe"), 900_000);

        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        let scan = LocalAdapter { folders: vec![games] }.scan(&env);
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Executable { path, .. } => assert!(path.ends_with("start.exe")),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn a_folder_with_no_executable_is_not_a_game() {
        let tmp = tempfile::tempdir().unwrap();
        let games = tmp.path().join("Games");
        fs::create_dir_all(games.join("Screenshots")).unwrap();
        fs::write(games.join("Screenshots/a.png"), b"x").unwrap();

        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        assert!(LocalAdapter { folders: vec![games] }.scan(&env).games.is_empty());
    }

    #[test]
    fn folder_names_are_tidied_into_titles() {
        assert_eq!(tidy_name("hollow_knight"), "Hollow Knight");
        assert_eq!(tidy_name("Doom-Eternal"), "Doom Eternal");
    }

    #[test]
    fn no_configured_folders_means_the_source_is_simply_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        assert!(!LocalAdapter { folders: vec![] }.scan(&env).status.detected);
    }

    #[test]
    fn ids_are_stable_when_the_folder_moves_to_another_drive() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("C_Games");
        let second = tmp.path().join("D_Games");
        write(&first.join("Celeste/Celeste.exe"), 10);
        write(&second.join("Celeste/Celeste.exe"), 10);

        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        let a = LocalAdapter { folders: vec![first] }.scan(&env);
        let b = LocalAdapter { folders: vec![second] }.scan(&env);
        assert_eq!(a.games[0].id, b.games[0].id);
    }
}
