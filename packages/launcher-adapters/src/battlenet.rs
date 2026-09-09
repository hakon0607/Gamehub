//! Battle.net.
//!
//! Blizzard's client keeps its game list in `product.db`, a protobuf file the
//! agent holds open and whose schema is not published. GameHub does not parse
//! it. Instead it reads the uninstall registry — every Battle.net game writes
//! an entry there with `Blizzard Entertainment` as publisher and an uninstall
//! string containing `--uid=<product uid>`, which is exactly the id the
//! `battlenet://` handler takes.
//!
//! This is the safest legitimate method available. Its limit: a game installed
//! but never registered (a manually copied folder) is not found.

use std::path::PathBuf;

use gamehub_detect::{
    model::{Game, GameSource, LaunchMethod},
    Env,
};

use crate::{AdapterScan, LauncherAdapter};

pub struct BattleNetAdapter;

impl LauncherAdapter for BattleNetAdapter {
    fn source(&self) -> GameSource {
        GameSource::Battlenet
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let client = crate::first_existing([
            env.folders.program_files_x86.join("Battle.net"),
            env.folders.program_files.join("Battle.net"),
        ]);

        let mut games = Vec::new();
        for entry in crate::uninstall::entries(env) {
            if !entry.publisher.to_lowercase().contains("blizzard") {
                continue;
            }
            // The client itself is not a game.
            if entry.display_name.eq_ignore_ascii_case("Battle.net") {
                continue;
            }

            let uid = product_uid(&entry.uninstall_string);
            let mut game = Game::new(GameSource::Battlenet, &entry.key, entry.display_name.trim());
            game.install_dir = (!entry.install_location.is_empty()).then(|| entry.install_location.clone());
            game.installed = game
                .install_dir
                .as_ref()
                .map(|d| PathBuf::from(d).exists())
                .unwrap_or(true);
            game.launch = Some(match &uid {
                Some(uid) => LaunchMethod::Uri {
                    uri: format!("battlenet://{uid}"),
                },
                None => LaunchMethod::Uri {
                    uri: "battlenet://".to_string(),
                },
            });
            games.push(game);
        }

        if client.is_none() && games.is_empty() {
            return AdapterScan::missing(GameSource::Battlenet);
        }

        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        AdapterScan::found(GameSource::Battlenet, client.map(|p| p.to_string_lossy().into_owned()), games)
            .with_limitation(
                "Battle.net keeps its game list in a database GameHub does not read. Games are found \
                 through their Windows uninstall entries, which covers everything installed by the \
                 Battle.net client itself.",
            )
    }
}

/// Pulls `--uid=wow` out of an uninstall command line.
fn product_uid(uninstall_string: &str) -> Option<String> {
    let start = uninstall_string.find("--uid=")? + "--uid=".len();
    let rest = &uninstall_string[start..];
    let uid: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    // Blizzard suffixes region variants, e.g. `wow_enus`; the base product is
    // what the protocol handler expects.
    let uid = uid.split('_').next().unwrap_or(&uid).to_string();
    (!uid.is_empty()).then_some(uid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::{env::Hive, FakeRegistry};

    #[test]
    fn extracts_the_product_uid_from_the_uninstall_command() {
        assert_eq!(
            product_uid("\"C:\\ProgramData\\Battle.net\\Agent\\Blizzard Uninstaller.exe\" --lang=enus --uid=wow_enus"),
            Some("wow".into())
        );
        assert_eq!(product_uid("nothing here"), None);
    }

    #[test]
    fn finds_blizzard_games_and_skips_the_client() {
        let tmp = tempfile::tempdir().unwrap();
        let base = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
        let registry = FakeRegistry::new()
            .with(Hive::LocalMachine, &format!("{base}\\WoW"), "DisplayName", "World of Warcraft")
            .with(Hive::LocalMachine, &format!("{base}\\WoW"), "Publisher", "Blizzard Entertainment")
            .with(Hive::LocalMachine, &format!("{base}\\WoW"), "UninstallString", "uninst.exe --uid=wow_enus")
            .with(Hive::LocalMachine, &format!("{base}\\Bnet"), "DisplayName", "Battle.net")
            .with(Hive::LocalMachine, &format!("{base}\\Bnet"), "Publisher", "Blizzard Entertainment");

        let env = Env::fixture(tmp.path(), registry);
        let scan = BattleNetAdapter.scan(&env);
        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].name, "World of Warcraft");
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Uri { uri } => assert_eq!(uri, "battlenet://wow"),
            other => panic!("expected a battlenet URI, got {other:?}"),
        }
    }
}
