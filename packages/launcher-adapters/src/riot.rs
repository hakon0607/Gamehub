//! Riot Games.
//!
//! Riot writes `%ProgramData%\Riot Games\RiotClientInstalls.json`, which points
//! at `RiotClientServices.exe`, and one settings file per installed product
//! under `%ProgramData%\Riot Games\Metadata\<product>.<patchline>\`. Both are
//! plain files Riot's own client reads, and together they give the product
//! name, the install path and the arguments that start each game.
//!
//! Riot has no URI handler, so this adapter launches `RiotClientServices.exe`
//! with the documented `--launch-product` arguments — the same command the
//! desktop shortcuts Riot creates contain.

use std::path::PathBuf;

use gamehub_detect::{
    model::{Game, GameSource, LaunchMethod},
    safepath, Env,
};

use crate::{AdapterScan, LauncherAdapter};

pub struct RiotAdapter;

/// Product ids Riot uses, with the names players know them by.
const PRODUCT_NAMES: &[(&str, &str)] = &[
    ("league_of_legends", "League of Legends"),
    ("valorant", "VALORANT"),
    ("bacon", "Legends of Runeterra"),
    ("wildrift", "Wild Rift"),
];

impl LauncherAdapter for RiotAdapter {
    fn source(&self) -> GameSource {
        GameSource::Riot
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let riot_root = env.folders.program_data.join("Riot Games");
        let Some(client) = riot_client(&riot_root) else {
            return AdapterScan::missing(GameSource::Riot);
        };

        let metadata = riot_root.join("Metadata");
        let mut games = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&metadata) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().into_owned();
                // Directories are named "<product>.<patchline>", e.g.
                // "league_of_legends.live".
                let Some((product, patchline)) = dir_name.split_once('.') else {
                    continue;
                };
                if !product.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    continue;
                }

                let name = PRODUCT_NAMES
                    .iter()
                    .find(|(id, _)| *id == product)
                    .map(|(_, label)| (*label).to_string())
                    .unwrap_or_else(|| product.replace('_', " "));

                let mut game = Game::new(GameSource::Riot, product, name);
                game.install_dir = install_path(&entry.path());
                game.installed = game
                    .install_dir
                    .as_ref()
                    .map(|p| PathBuf::from(p).exists())
                    .unwrap_or(false);
                game.launch = Some(LaunchMethod::Executable {
                    path: client.to_string_lossy().into_owned(),
                    args: vec![
                        format!("--launch-product={product}"),
                        format!("--launch-patchline={patchline}"),
                    ],
                    working_dir: client.parent().map(|p| p.to_string_lossy().into_owned()),
                });
                games.push(game);
            }
        }

        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        games.dedup_by(|a, b| a.id == b.id);
        AdapterScan::found(GameSource::Riot, Some(riot_root.to_string_lossy().into_owned()), games)
    }
}

fn riot_client(riot_root: &std::path::Path) -> Option<PathBuf> {
    let installs = riot_root.join("RiotClientInstalls.json");
    let text = std::fs::read_to_string(&installs).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;

    for key in ["rc_live", "rc_default", "rc_beta"] {
        if let Some(path) = json.get(key).and_then(|v| v.as_str()) {
            let path = PathBuf::from(path);
            // The client is an executable GameHub will run, so it goes through
            // the same validation as any game binary.
            if let Some(parent) = path.parent() {
                if safepath::validate_executable(&path, &[parent.to_path_buf()]).is_ok() {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// The install path recorded in the product settings file. Riot writes YAML
/// here; only one key is needed, so it is read line by line rather than by
/// adding a YAML parser for a single field.
fn install_path(metadata_dir: &std::path::Path) -> Option<String> {
    let entries = std::fs::read_dir(metadata_dir).ok()?;
    for entry in entries.flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let text = std::fs::read_to_string(entry.path()).ok()?;
        for line in text.lines() {
            if let Some(value) = line.trim().strip_prefix("product_install_full_path:") {
                let value = value.trim().trim_matches(['"', '\'']);
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::FakeRegistry;
    use std::fs;

    #[test]
    fn finds_products_and_builds_the_documented_launch_command() {
        let tmp = tempfile::tempdir().unwrap();
        let riot = tmp.path().join("ProgramData/Riot Games");
        let client_dir = tmp.path().join("Riot Client");
        fs::create_dir_all(&client_dir).unwrap();
        let client = client_dir.join("RiotClientServices.exe");
        fs::write(&client, b"MZ").unwrap();

        let game_dir = tmp.path().join("Riot Games/League of Legends");
        fs::create_dir_all(&game_dir).unwrap();

        let meta = riot.join("Metadata/league_of_legends.live");
        fs::create_dir_all(&meta).unwrap();
        fs::write(
            meta.join("league_of_legends.live.product_settings.yaml"),
            format!("product_install_full_path: \"{}\"\n", game_dir.to_string_lossy().replace('\\', "/")),
        )
        .unwrap();
        fs::create_dir_all(&riot).unwrap();
        fs::write(
            riot.join("RiotClientInstalls.json"),
            format!("{{\"rc_live\": \"{}\"}}", client.to_string_lossy().replace('\\', "\\\\")),
        )
        .unwrap();

        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        let scan = RiotAdapter.scan(&env);
        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].name, "League of Legends");
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Executable { path, args, .. } => {
                assert!(path.ends_with("RiotClientServices.exe"));
                assert_eq!(args, &["--launch-product=league_of_legends", "--launch-patchline=live"]);
            }
            other => panic!("expected an executable launch, got {other:?}"),
        }
    }

    #[test]
    fn a_client_path_that_does_not_exist_means_riot_is_not_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let riot = tmp.path().join("ProgramData/Riot Games");
        fs::create_dir_all(&riot).unwrap();
        fs::write(riot.join("RiotClientInstalls.json"), r#"{"rc_live": "C:\\nope\\x.exe"}"#).unwrap();
        let env = Env::fixture(tmp.path(), FakeRegistry::new());
        assert!(!RiotAdapter.scan(&env).status.detected);
    }
}
