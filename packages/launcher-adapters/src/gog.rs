//! GOG.
//!
//! Every GOG installer — Galaxy or the standalone offline installer — writes a
//! key under `SOFTWARE\GOG.com\Games\<productId>` holding the game's name, its
//! path and its executable. Reading that covers both cases and needs no access
//! to Galaxy's own database, which is a SQLite file the client keeps open.
//!
//! GOG games are DRM-free, so launching the executable directly is the normal
//! and supported way to start them. When Galaxy is installed GameHub still
//! prefers Galaxy's handler, so that playtime and achievements are recorded.

use std::path::PathBuf;

use gamehub_detect::{
    env::Hive,
    model::{Game, GameSource, LaunchMethod},
    safepath, Env,
};

use crate::{first_existing, AdapterScan, LauncherAdapter};

pub struct GogAdapter;

const GAMES_KEY: &str = "SOFTWARE\\GOG.com\\Games";

impl LauncherAdapter for GogAdapter {
    fn source(&self) -> GameSource {
        GameSource::Gog
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let galaxy = galaxy_path(env);
        let product_ids = env.subkeys_both_views(Hive::LocalMachine, GAMES_KEY);

        if product_ids.is_empty() && galaxy.is_none() {
            return AdapterScan::missing(GameSource::Gog);
        }

        let mut games = Vec::new();
        for id in product_ids {
            let key = format!("{GAMES_KEY}\\{id}");
            let read = |value: &str| env.reg_string_both_views(Hive::LocalMachine, &key, value);

            let Some(name) = read("gameName").or_else(|| read("startMenu")) else {
                continue;
            };
            let Some(path) = read("path") else { continue };
            let install = PathBuf::from(&path);

            let mut game = Game::new(GameSource::Gog, &id, name.trim());
            game.installed = install.exists();
            game.install_dir = Some(path.clone());

            game.launch = if galaxy.is_some() {
                Some(LaunchMethod::Uri {
                    uri: format!("goggalaxy://openGameView/{id}"),
                })
            } else {
                // No Galaxy: run the executable the installer registered, but
                // only after it has passed the same validation as any other path.
                read("exe")
                    .map(PathBuf::from)
                    .filter(|exe| safepath::validate_executable(exe, &[install.clone()]).is_ok())
                    .map(|exe| LaunchMethod::Executable {
                        path: exe.to_string_lossy().into_owned(),
                        args: Vec::new(),
                        working_dir: Some(path.clone()),
                    })
            };

            games.push(game);
        }

        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let scan = AdapterScan::found(
            GameSource::Gog,
            galaxy.map(|p| p.to_string_lossy().into_owned()),
            games,
        );
        scan.with_limitation(
            "GOG games are found through the registry keys their installers write. A game copied \
             from another machine without being installed will not appear — add its folder under \
             Settings › Game folders.",
        )
    }
}

fn galaxy_path(env: &Env) -> Option<PathBuf> {
    env.reg_string_both_views(Hive::LocalMachine, "SOFTWARE\\GOG.com\\GalaxyClient\\paths", "client")
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .or_else(|| first_existing([env.folders.program_files_x86.join("GOG Galaxy")]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::FakeRegistry;
    use std::fs;

    #[test]
    fn reads_games_from_both_registry_views() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("GOG/Witcher 3");
        fs::create_dir_all(&install).unwrap();
        fs::write(install.join("witcher3.exe"), b"MZ").unwrap();

        let registry = FakeRegistry::new()
            .with(Hive::LocalMachine, "SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1207658924", "gameName", "The Witcher 3")
            .with(
                Hive::LocalMachine,
                "SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1207658924",
                "path",
                &install.to_string_lossy(),
            )
            .with(
                Hive::LocalMachine,
                "SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1207658924",
                "exe",
                &install.join("witcher3.exe").to_string_lossy(),
            );

        let env = Env::fixture(tmp.path(), registry);
        let scan = GogAdapter.scan(&env);
        assert!(scan.status.detected);
        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].name, "The Witcher 3");
        assert!(scan.games[0].installed);
    }

    #[test]
    fn without_galaxy_it_runs_the_registered_executable_and_validates_it() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("GOG/Game");
        fs::create_dir_all(&install).unwrap();
        fs::write(install.join("game.exe"), b"MZ").unwrap();

        let base = "SOFTWARE\\GOG.com\\Games\\1";
        let registry = FakeRegistry::new()
            .with(Hive::LocalMachine, base, "gameName", "Game")
            .with(Hive::LocalMachine, base, "path", &install.to_string_lossy())
            .with(Hive::LocalMachine, base, "exe", &install.join("game.exe").to_string_lossy());

        let env = Env::fixture(tmp.path(), registry);
        let scan = GogAdapter.scan(&env);
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Executable { path, .. } => assert!(path.ends_with("game.exe")),
            other => panic!("expected an executable launch without Galaxy, got {other:?}"),
        }
    }

    #[test]
    fn an_executable_outside_the_install_folder_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("GOG/Game");
        fs::create_dir_all(&install).unwrap();
        let elsewhere = tmp.path().join("evil.exe");
        fs::write(&elsewhere, b"MZ").unwrap();

        let base = "SOFTWARE\\GOG.com\\Games\\1";
        let registry = FakeRegistry::new()
            .with(Hive::LocalMachine, base, "gameName", "Game")
            .with(Hive::LocalMachine, base, "path", &install.to_string_lossy())
            .with(Hive::LocalMachine, base, "exe", &elsewhere.to_string_lossy());

        let env = Env::fixture(tmp.path(), registry);
        let scan = GogAdapter.scan(&env);
        assert!(scan.games[0].launch.is_none(), "a path outside the game folder must not become a launch method");
    }

    #[test]
    fn the_limitation_is_reported_rather_than_hidden() {
        let tmp = tempfile::tempdir().unwrap();
        let registry = FakeRegistry::new()
            .with(Hive::LocalMachine, "SOFTWARE\\GOG.com\\Games\\1", "gameName", "Game")
            .with(Hive::LocalMachine, "SOFTWARE\\GOG.com\\Games\\1", "path", "/nope");
        let env = Env::fixture(tmp.path(), registry);
        assert!(GogAdapter.scan(&env).status.limitation.is_some());
    }
}
