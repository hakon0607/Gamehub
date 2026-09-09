//! Ubisoft Connect.
//!
//! Ubisoft records installed games under
//! `SOFTWARE\Ubisoft\Launcher\Installs\<installId>` with an `InstallDir`. The
//! registry does not hold the game's title, so the name is taken from the
//! install folder, which Ubisoft names after the game. That is a real
//! limitation and it is reported rather than papered over.
//!
//! Launching uses `uplay://launch/<installId>/0`, Ubisoft Connect's own handler.

use std::path::PathBuf;

use gamehub_detect::{
    env::Hive,
    model::{Game, GameSource, LaunchMethod},
    Env,
};

use crate::{first_existing, AdapterScan, LauncherAdapter};

pub struct UbisoftAdapter;

const INSTALLS_KEY: &str = "SOFTWARE\\Ubisoft\\Launcher\\Installs";

impl LauncherAdapter for UbisoftAdapter {
    fn source(&self) -> GameSource {
        GameSource::Ubisoft
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let launcher = launcher_path(env);
        let install_ids = env.subkeys_both_views(Hive::LocalMachine, INSTALLS_KEY);
        if launcher.is_none() && install_ids.is_empty() {
            return AdapterScan::missing(GameSource::Ubisoft);
        }

        let mut games = Vec::new();
        for id in install_ids {
            let key = format!("{INSTALLS_KEY}\\{id}");
            let Some(dir) = env.reg_string_both_views(Hive::LocalMachine, &key, "InstallDir") else {
                continue;
            };
            let path = PathBuf::from(&dir);
            let name = pretty_name_from_dir(&path).unwrap_or_else(|| format!("Ubisoft game {id}"));

            let mut game = Game::new(GameSource::Ubisoft, &id, name);
            game.installed = path.exists();
            game.install_dir = Some(path.to_string_lossy().into_owned());
            game.launch = Some(LaunchMethod::Uri {
                uri: format!("uplay://launch/{id}/0"),
            });
            games.push(game);
        }

        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        AdapterScan::found(
            GameSource::Ubisoft,
            launcher.map(|p| p.to_string_lossy().into_owned()),
            games,
        )
        .with_limitation(
            "Ubisoft Connect does not store game titles where GameHub can read them, so names come \
             from the install folder. Rename any that look wrong — the new name is kept.",
        )
    }
}

/// "Assassins Creed Valhalla" from `D:\Ubisoft\games\Assassins Creed Valhalla`.
fn pretty_name_from_dir(path: &std::path::Path) -> Option<String> {
    let raw = path.file_name()?.to_string_lossy();
    let cleaned = raw.replace(['_', '-'], " ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    (!cleaned.is_empty()).then_some(cleaned)
}

fn launcher_path(env: &Env) -> Option<PathBuf> {
    env.reg_string_both_views(Hive::LocalMachine, "SOFTWARE\\Ubisoft\\Launcher", "InstallDir")
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .or_else(|| first_existing([env.folders.program_files_x86.join("Ubisoft/Ubisoft Game Launcher")]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::FakeRegistry;
    use std::fs;

    #[test]
    fn names_games_from_their_install_folder_and_launches_through_uplay() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("Ubisoft/games/Assassins_Creed_Valhalla");
        fs::create_dir_all(&install).unwrap();

        let registry = FakeRegistry::new().with(
            Hive::LocalMachine,
            "SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher\\Installs\\5595",
            "InstallDir",
            &install.to_string_lossy(),
        );
        let env = Env::fixture(tmp.path(), registry);
        let scan = UbisoftAdapter.scan(&env);

        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].name, "Assassins Creed Valhalla");
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Uri { uri } => assert_eq!(uri, "uplay://launch/5595/0"),
            other => panic!("expected uplay URI, got {other:?}"),
        }
        assert!(scan.status.limitation.is_some(), "the naming limitation must be visible to the user");
    }
}
