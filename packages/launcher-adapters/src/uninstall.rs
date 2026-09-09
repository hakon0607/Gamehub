//! Windows' uninstall registry.
//!
//! Several launchers do not publish a game list of their own, but every
//! installer writes here. It gives a display name, a publisher and usually an
//! install location — enough to find a launcher's games and no more than the
//! Programs and Features control panel already shows.

use gamehub_detect::{env::Hive, Env};

const KEYS: &[(Hive, &str)] = &[
    (Hive::LocalMachine, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
    (Hive::LocalMachine, "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
    (Hive::CurrentUser, "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
];

#[derive(Debug, Clone)]
pub struct UninstallEntry {
    /// The registry subkey name, which is stable and unique per installed app.
    pub key: String,
    pub display_name: String,
    pub publisher: String,
    pub install_location: String,
    pub uninstall_string: String,
}

pub fn entries(env: &Env) -> Vec<UninstallEntry> {
    let mut out = Vec::new();
    for (hive, base) in KEYS {
        for key in env.registry.subkeys(*hive, base) {
            let path = format!("{base}\\{key}");
            let read = |value: &str| env.registry.read_string(*hive, &path, value).unwrap_or_default();
            let display_name = read("DisplayName");
            if display_name.trim().is_empty() {
                continue;
            }
            out.push(UninstallEntry {
                key,
                display_name,
                publisher: read("Publisher"),
                install_location: read("InstallLocation"),
                uninstall_string: read("UninstallString"),
            });
        }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out.dedup_by(|a, b| a.key == b.key);
    out
}
