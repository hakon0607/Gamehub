//! Epic Games Store.
//!
//! The launcher writes one JSON manifest per installed item to
//! `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests`. That directory is the
//! whole story: it holds the display name, the install location, the size and
//! the `AppName` needed to start the game through Epic's own handler.
//!
//! The subtlety is that DLC, Unreal Engine itself and plugins all get manifests
//! too. A manifest whose `MainGameAppName` differs from its `AppName` is an
//! add-on to something else, and listing those would fill the library with
//! entries the user cannot launch.

use std::path::{Path, PathBuf};

use gamehub_detect::{
    model::{Game, GameSource, LaunchMethod},
    Env,
};
use serde::Deserialize;

use crate::{AdapterScan, LauncherAdapter};

pub struct EpicAdapter;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EpicManifest {
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    install_location: String,
    #[serde(default)]
    launch_executable: String,
    #[serde(default)]
    app_name: String,
    #[serde(default)]
    main_game_app_name: String,
    #[serde(default)]
    install_size: u64,
    #[serde(default)]
    app_categories: Vec<String>,
    #[serde(default, rename = "bIsIncompleteInstall")]
    is_incomplete_install: bool,
}

pub fn manifest_dir(env: &Env) -> PathBuf {
    env.folders
        .program_data
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests")
}

impl LauncherAdapter for EpicAdapter {
    fn source(&self) -> GameSource {
        GameSource::Epic
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let dir = manifest_dir(env);
        if !dir.is_dir() {
            return AdapterScan::missing(GameSource::Epic);
        }

        let mut games = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) != Some("item") {
                    continue;
                }
                if let Some(game) = read_manifest(&entry.path()) {
                    games.push(game);
                }
            }
        }
        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        AdapterScan::found(GameSource::Epic, Some(dir.to_string_lossy().into_owned()), games)
    }
}

fn read_manifest(path: &Path) -> Option<Game> {
    let text = std::fs::read_to_string(path).ok()?;
    let manifest: EpicManifest = serde_json::from_str(&text).ok()?;

    if manifest.display_name.trim().is_empty() || manifest.app_name.trim().is_empty() {
        return None;
    }

    // An add-on: its parent game is the thing the user launches.
    if !manifest.main_game_app_name.is_empty() && manifest.main_game_app_name != manifest.app_name {
        return None;
    }

    // Engines, plugins and asset packs are shipped through the same launcher.
    let is_application = manifest.app_categories.is_empty()
        || manifest.app_categories.iter().any(|c| c.eq_ignore_ascii_case("games"));
    if !is_application {
        return None;
    }
    if manifest.launch_executable.trim().is_empty() {
        // Engine and plugin manifests have no executable of their own.
        return None;
    }

    let mut game = Game::new(GameSource::Epic, &manifest.app_name, manifest.display_name.trim());
    game.installed = !manifest.is_incomplete_install;

    if !manifest.install_location.is_empty() {
        let location = PathBuf::from(&manifest.install_location);
        if location.exists() {
            game.install_dir = Some(location.to_string_lossy().into_owned());
        } else {
            game.installed = false;
        }
    }

    game.size_bytes = (manifest.install_size > 0).then_some(manifest.install_size);
    game.launch = Some(LaunchMethod::Uri {
        uri: format!(
            "com.epicgames.launcher://apps/{}?action=launch&silent=true",
            manifest.app_name
        ),
    });

    Some(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::FakeRegistry;
    use std::fs;

    fn write_manifest(dir: &Path, file: &str, json: &str) {
        fs::write(dir.join(file), json).unwrap();
    }

    fn fixture() -> (tempfile::TempDir, Env) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let manifests = root.join("ProgramData/Epic/EpicGamesLauncher/Data/Manifests");
        fs::create_dir_all(&manifests).unwrap();
        let install = root.join("Games/Fortnite");
        fs::create_dir_all(&install).unwrap();

        write_manifest(
            &manifests,
            "1.item",
            &format!(
                r#"{{"DisplayName":"Fortnite","InstallLocation":"{}","LaunchExecutable":"FortniteGame/Binaries/Win64/FortniteClient-Win64-Shipping.exe","AppName":"Fortnite","MainGameAppName":"Fortnite","InstallSize":38000000000,"AppCategories":["games","applications"],"bIsIncompleteInstall":false}}"#,
                install.to_string_lossy().replace('\\', "\\\\")
            ),
        );
        // A DLC for the same game.
        write_manifest(
            &manifests,
            "2.item",
            r#"{"DisplayName":"Fortnite Skin Pack","InstallLocation":"","LaunchExecutable":"x.exe","AppName":"FortniteDLC","MainGameAppName":"Fortnite","AppCategories":["games"]}"#,
        );
        // Unreal Engine, which has no launch executable.
        write_manifest(
            &manifests,
            "3.item",
            r#"{"DisplayName":"Unreal Engine","InstallLocation":"","LaunchExecutable":"","AppName":"UE_5.4","AppCategories":["engines"]}"#,
        );

        let env = Env::fixture(root, FakeRegistry::new());
        (tmp, env)
    }

    #[test]
    fn reads_the_manifest_directory() {
        let (_t, env) = fixture();
        let scan = EpicAdapter.scan(&env);
        assert!(scan.status.detected);
        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].name, "Fortnite");
        assert_eq!(scan.games[0].size_bytes, Some(38_000_000_000));
    }

    #[test]
    fn skips_dlc_and_the_engine() {
        let (_t, env) = fixture();
        let names: Vec<String> = EpicAdapter.scan(&env).games.iter().map(|g| g.name.clone()).collect();
        assert!(!names.iter().any(|n| n.contains("Skin Pack")));
        assert!(!names.iter().any(|n| n.contains("Unreal Engine")));
    }

    #[test]
    fn launches_through_the_epic_handler() {
        let (_t, env) = fixture();
        let scan = EpicAdapter.scan(&env);
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Uri { uri } => {
                assert_eq!(uri, "com.epicgames.launcher://apps/Fortnite?action=launch&silent=true");
            }
            other => panic!("expected a URI launch, got {other:?}"),
        }
    }

    #[test]
    fn a_malformed_manifest_is_skipped_not_fatal() {
        let (t, env) = fixture();
        fs::write(
            t.path().join("ProgramData/Epic/EpicGamesLauncher/Data/Manifests/broken.item"),
            "{ not json",
        )
        .unwrap();
        assert_eq!(EpicAdapter.scan(&env).games.len(), 1);
    }

    #[test]
    fn an_install_location_that_is_gone_means_not_installed() {
        let (t, env) = fixture();
        fs::remove_dir_all(t.path().join("Games/Fortnite")).unwrap();
        let scan = EpicAdapter.scan(&env);
        assert!(!scan.games[0].installed);
    }

    #[test]
    fn epic_missing_is_a_normal_result() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        assert!(!EpicAdapter.scan(&env).status.detected);
    }
}
