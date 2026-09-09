//! Xbox / Microsoft Store (the Xbox app, "PC Game Pass").
//!
//! Store games are MSIX packages installed into `WindowsApps`, a directory
//! whose ACL blocks ordinary reads — so walking it is not an option, and any
//! adapter that claims to would fail on a real machine. What *is* readable is
//! the Gaming Services package repository under
//! `HKLM\SOFTWARE\Microsoft\GamingServices\PackageRepository\Root`, which the
//! Xbox app writes for every game it installs, and which contains the package
//! family name and the install root.
//!
//! Launching uses the shell's AppsFolder entry, `shell:AppsFolder\<PFN>!App`,
//! the same mechanism the Start menu uses. `App` is the application id for the
//! overwhelming majority of game packages; where a game uses a different one,
//! it is recorded in the package's own manifest and can be overridden per game
//! in the UI. That override, rather than a wrong guess, is the honest handling.

use gamehub_detect::{
    env::Hive,
    model::{Game, GameSource, LaunchMethod},
    Env,
};

use crate::{AdapterScan, LauncherAdapter};

pub struct XboxAdapter;

const REPOSITORY: &str = "SOFTWARE\\Microsoft\\GamingServices\\PackageRepository\\Root";
const GAMING_SERVICES: &str = "SOFTWARE\\Microsoft\\GamingServices";

impl LauncherAdapter for XboxAdapter {
    fn source(&self) -> GameSource {
        GameSource::Xbox
    }

    fn scan(&self, env: &Env) -> AdapterScan {
        let installed = env
            .reg_string(Hive::LocalMachine, GAMING_SERVICES, "InstallPath")
            .or_else(|| env.reg_string(Hive::LocalMachine, REPOSITORY, ""));
        let roots = env.registry.subkeys(Hive::LocalMachine, REPOSITORY);

        if installed.is_none() && roots.is_empty() {
            return AdapterScan::missing(GameSource::Xbox);
        }

        let mut games = Vec::new();
        for product in roots {
            let product_key = format!("{REPOSITORY}\\{product}");
            // Each product holds one subkey per installed package version.
            for package in env.registry.subkeys(Hive::LocalMachine, &product_key) {
                let key = format!("{product_key}\\{package}");
                let read = |value: &str| env.registry.read_string(Hive::LocalMachine, &key, value);

                let Some(package_full_name) = read("Package").or(Some(package.clone())) else {
                    continue;
                };
                let family = package_family_name(&package_full_name);
                let root = read("Root").unwrap_or_default();

                // The install folder is the best title available: Xbox names it
                // after the game ("D:\\XboxGames\\Halo Infinite"), whereas the
                // package name is often nothing but a publisher id.
                let name = folder_title(&root).unwrap_or_else(|| display_name(&family));
                let mut game = Game::new(GameSource::Xbox, &family, name);
                game.install_dir = (!root.is_empty()).then_some(root);
                game.installed = true;
                game.launch = Some(LaunchMethod::Uwp {
                    app_user_model_id: format!("{family}!App"),
                });
                games.push(game);
            }
        }

        games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        games.dedup_by(|a, b| a.id == b.id);
        AdapterScan::found(GameSource::Xbox, installed, games)
            .with_limitation(
                "Windows protects the Store's install folder, so Xbox games are read from the Gaming \
                 Services registry. Titles come from the package name and are often abbreviated — \
                 rename them once and the name sticks.",
            )
    }
}

/// `Publisher.Game_1.2.3.0_x64__8wekyb3d8bbwe` → `Publisher.Game_8wekyb3d8bbwe`.
/// The family name is what the shell's AppsFolder is keyed by.
fn package_family_name(full_name: &str) -> String {
    let parts: Vec<&str> = full_name.split('_').collect();
    match (parts.first(), parts.last()) {
        (Some(name), Some(publisher)) if parts.len() >= 3 => format!("{name}_{publisher}"),
        _ => full_name.to_string(),
    }
}

/// The install folder's own name, when it looks like a title rather than an id.
fn folder_title(root: &str) -> Option<String> {
    // Split on both separators by hand: these are Windows paths, and on any
    // other platform `Path::file_name` would hand back the whole string.
    let name = root
        .trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()?
        .trim()
        .to_string();
    looks_like_a_title(&name).then_some(name)
}

/// Rejects the identifier-shaped strings that made "4297127D64EC6" show up in
/// the library as a game called "4297127 D64 EC6": a real title has letters,
/// is not mostly digits, and is not a bare hex blob.
fn looks_like_a_title(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 80 {
        return false;
    }
    let letters = trimmed.chars().filter(|c| c.is_alphabetic()).count();
    let digits = trimmed.chars().filter(|c| c.is_ascii_digit()).count();
    if letters < 3 || digits > letters {
        return false;
    }
    // The giveaway for an identifier: letters and digits run together with no
    // spaces at all. Real titles either have a space ("Forza Horizon 5") or are
    // a short single word ("Celeste"), never a long alphanumeric run.
    if !trimmed.contains(char::is_whitespace) && digits > 0 && trimmed.len() > 8 {
        return false;
    }
    true
}

/// Turns `Microsoft.HaloInfinite_8wekyb3d8bbwe` into "Halo Infinite". It is a
/// guess from an identifier, which is why the adapter reports the limitation
/// and the UI allows a rename.
fn display_name(family: &str) -> String {
    let stem = family.split('_').next().unwrap_or(family);
    // The last dot segment is the app name; but when a package has only a
    // publisher id and nothing else, that segment is the id itself, and a
    // title cannot be recovered from it.
    let last = stem.rsplit('.').next().unwrap_or(stem);
    if !looks_like_a_title(last) {
        return "Xbox game".to_string();
    }
    let chars: Vec<char> = last.chars().collect();
    let mut out = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        let previous = i.checked_sub(1).and_then(|j| chars.get(j)).copied();
        let next = chars.get(i + 1).copied();
        // A capital starts a new word only when it follows a lower-case letter
        // or a digit, or when it ends a run of capitals ("UWPGame" -> "UWP Game").
        let starts_word = ch.is_uppercase()
            && match (previous, next) {
                (Some(p), _) if p.is_lowercase() || p.is_ascii_digit() => true,
                (Some(p), Some(n)) if p.is_uppercase() && n.is_lowercase() => true,
                _ => false,
            };
        if starts_word && !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push(ch);
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::FakeRegistry;

    #[test]
    fn derives_the_family_name_from_the_full_package_name() {
        assert_eq!(
            package_family_name("Microsoft.HaloInfinite_1.0.0.0_x64__8wekyb3d8bbwe"),
            "Microsoft.HaloInfinite_8wekyb3d8bbwe"
        );
    }

    #[test]
    fn splits_camel_case_identifiers_into_readable_titles() {
        assert_eq!(display_name("Microsoft.HaloInfinite_8wekyb3d8bbwe"), "Halo Infinite");
        assert_eq!(display_name("Mojang.MinecraftUWP_8wekyb3d8bbwe"), "Minecraft UWP");
        assert_eq!(display_name("SomePublisher.UWPGame_abc"), "UWP Game");
        assert_eq!(display_name("Publisher.Forza5_abc"), "Forza5");
    }

    #[test]
    fn an_identifier_is_never_shown_as_a_game_title() {
        // The exact string that appeared in the library as "4297127 D64 EC6".
        assert!(!looks_like_a_title("4297127D64EC6"));
        assert!(!looks_like_a_title("8wekyb3d8bbwe"));
        assert!(!looks_like_a_title("a1"));
        assert!(looks_like_a_title("Halo Infinite"));
        assert!(looks_like_a_title("Forza Horizon 5"));
        assert_eq!(display_name("4297127D64EC6_8wekyb3d8bbwe"), "Xbox game");
    }

    #[test]
    fn the_install_folder_name_is_preferred_over_the_package_id() {
        let tmp = tempfile::tempdir().unwrap();
        let key = format!("{REPOSITORY}\\Sim\\4297127D64EC6.Supermarket_1.0.0.0_x64__abcdefghijklm");
        let registry = FakeRegistry::new()
            .with(Hive::LocalMachine, GAMING_SERVICES, "InstallPath", "C:\\GamingServices")
            .with(Hive::LocalMachine, &key, "Package", "4297127D64EC6.Supermarket_1.0.0.0_x64__abcdefghijklm")
            .with(Hive::LocalMachine, &key, "Root", "D:\\XboxGames\\Supermarket Simulator");

        let env = Env::fixture(tmp.path(), registry);
        let scan = XboxAdapter.scan(&env);
        assert_eq!(scan.games[0].name, "Supermarket Simulator");
    }

    #[test]
    fn reads_games_from_the_gaming_services_repository() {
        let tmp = tempfile::tempdir().unwrap();
        let key = format!("{REPOSITORY}\\Halo\\Microsoft.HaloInfinite_1.0.0.0_x64__8wekyb3d8bbwe");
        let registry = FakeRegistry::new()
            .with(Hive::LocalMachine, GAMING_SERVICES, "InstallPath", "C:\\Program Files\\GamingServices")
            .with(Hive::LocalMachine, &key, "Package", "Microsoft.HaloInfinite_1.0.0.0_x64__8wekyb3d8bbwe")
            .with(Hive::LocalMachine, &key, "Root", "D:\\XboxGames\\Halo Infinite");

        let env = Env::fixture(tmp.path(), registry);
        let scan = XboxAdapter.scan(&env);
        assert!(scan.status.detected);
        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].name, "Halo Infinite");
        match scan.games[0].launch.as_ref().unwrap() {
            LaunchMethod::Uwp { app_user_model_id } => {
                assert_eq!(app_user_model_id, "Microsoft.HaloInfinite_8wekyb3d8bbwe!App");
            }
            other => panic!("expected a UWP launch, got {other:?}"),
        }
        assert!(scan.status.limitation.is_some());
    }
}
