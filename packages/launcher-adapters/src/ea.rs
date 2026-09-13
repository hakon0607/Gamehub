//! EA (the EA app, formerly Origin).
//!
//! EA does not publish a manifest directory the way Epic does. What every EA
//! game does have is an `__Installer\installerdata.xml` inside its install
//! folder containing the content id, and an entry in Windows' uninstall
//! registry with the publisher set to Electronic Arts. GameHub combines the
//! two: the uninstall entry gives the name and the folder, the XML gives the id
//! that `origin2://game/launch?offerIds=<id>` needs.
//!
//! When the content id cannot be read, the game is still listed and still
//! launches — through the EA app's library page rather than straight into the
//! game. That is a smaller capability, stated plainly, rather than a launch
//! button that silently does nothing.

use std::path::{Path, PathBuf};

use gamehub_detect::{
    model::{Game, GameSource, LaunchMethod},
    Env,
};

use crate::{uninstall::UninstallEntry, AdapterScan, LauncherAdapter};

pub struct EaAdapter;

const PUBLISHERS: &[&str] = &["electronic arts", "ea games", "ea sports"];

impl LauncherAdapter for EaAdapter {
    fn source(&self) -> GameSource {
        GameSource::Ea
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let app = crate::first_existing([
            env.folders.program_files.join("Electronic Arts/EA Desktop"),
            env.folders.program_files_x86.join("Origin"),
        ]);

        let entries = crate::uninstall::entries(env);
        let mine: Vec<&UninstallEntry> = entries
            .iter()
            .filter(|e| {
                let publisher = e.publisher.to_lowercase();
                PUBLISHERS.iter().any(|p| publisher.contains(p))
            })
            .collect();

        if app.is_none() && mine.is_empty() {
            return AdapterScan::missing(GameSource::Ea);
        }

        let mut games = Vec::new();
        let mut missing_ids = 0usize;
        for entry in mine {
            let install = PathBuf::from(&entry.install_location);
            let mut game = Game::new(GameSource::Ea, &entry.key, entry.display_name.trim());
            game.installed = install.exists();
            game.install_dir = (!entry.install_location.is_empty()).then(|| entry.install_location.clone());

            match content_id(&install) {
                Some(id) => {
                    game.launch = Some(LaunchMethod::Uri {
                        uri: format!("origin2://game/launch?offerIds={id}"),
                    });
                }
                None => {
                    missing_ids += 1;
                    game.launch = Some(LaunchMethod::Uri {
                        uri: "origin2://library/open".to_string(),
                    });
                }
            }
            games.push(game);
        }

        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let scan = AdapterScan::found(GameSource::Ea, app.map(|p| p.to_string_lossy().into_owned()), games);
        if missing_ids > 0 {
            scan.with_limitation(format!(
                "{missing_ids} EA game(s) do not expose a content id, so Play opens the EA app's \
                 library instead of starting the game directly."
            ))
        } else {
            scan
        }
    }
}

/// Reads the offer id out of `__Installer\installerdata.xml`. The file is XML,
/// but the only thing needed from it is one attribute, so it is matched
/// textually rather than by pulling in an XML parser for a single field.
fn content_id(install_dir: &Path) -> Option<String> {
    let path = install_dir.join("__Installer").join("installerdata.xml");
    let text = std::fs::read_to_string(path).ok()?;
    let start = text.find("<contentID>")? + "<contentID>".len();
    let end = text[start..].find("</contentID>")? + start;
    let id = text[start..end].trim();
    let valid = !id.is_empty()
        && id.len() < 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
    valid.then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::{env::Hive, FakeRegistry};
    use std::fs;

    fn registry_with_game(key: &str, name: &str, publisher: &str, location: &str) -> FakeRegistry {
        let path = format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{key}");
        FakeRegistry::new()
            .with(Hive::LocalMachine, &path, "DisplayName", name)
            .with(Hive::LocalMachine, &path, "Publisher", publisher)
            .with(Hive::LocalMachine, &path, "InstallLocation", location)
    }

    #[test]
    fn finds_ea_games_and_reads_the_content_id() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("EA Games/Battlefield");
        fs::create_dir_all(install.join("__Installer")).unwrap();
        fs::write(
            install.join("__Installer/installerdata.xml"),
            "<DiPManifest><contentIDs><contentID>1039093</contentID></contentIDs></DiPManifest>",
        )
        .unwrap();

        let env = Env::fixture(
            tmp.path(),
            registry_with_game("Battlefield", "Battlefield 2042", "Electronic Arts", &install.to_string_lossy()),
        );
        let scan = EaAdapter.scan(&env);
        assert_eq!(scan.games.len(), 1);
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Uri { uri } => assert_eq!(uri, "origin2://game/launch?offerIds=1039093"),
            other => panic!("expected an offer launch, got {other:?}"),
        }
        assert!(scan.status.limitation.is_none());
    }

    #[test]
    fn a_game_without_a_content_id_still_appears_and_says_so() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("EA Games/Sims");
        fs::create_dir_all(&install).unwrap();
        let env = Env::fixture(
            tmp.path(),
            registry_with_game("Sims", "The Sims 4", "Electronic Arts", &install.to_string_lossy()),
        );
        let scan = EaAdapter.scan(&env);
        assert_eq!(scan.games.len(), 1);
        assert!(scan.status.limitation.as_ref().unwrap().contains("content id"));
    }

    #[test]
    fn ignores_uninstall_entries_from_other_publishers() {
        let tmp = tempfile::tempdir().unwrap();
        let env = Env::fixture(
            tmp.path(),
            registry_with_game("Notepad", "Notepad++", "Don Ho", &tmp.path().to_string_lossy()),
        );
        assert!(EaAdapter.scan(&env).games.is_empty());
    }

    #[test]
    fn a_content_id_with_junk_in_it_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("g");
        fs::create_dir_all(install.join("__Installer")).unwrap();
        fs::write(
            install.join("__Installer/installerdata.xml"),
            "<contentID>123\" & calc.exe</contentID>",
        )
        .unwrap();
        assert_eq!(content_id(&install), None);
    }
}
