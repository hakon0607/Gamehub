//! The machine an adapter reads from.
//!
//! Every adapter takes an `Env` rather than touching `std::env` or the registry
//! directly. On Windows the real implementation reads the real registry and the
//! real known folders; in tests the same adapters run against a fixture tree and
//! a hand-written registry map. That is the only reason the launcher detection
//! in this project can be tested at all on a machine that is not Windows.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The two registry hives GameHub ever reads. It never writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

impl Hive {
    pub fn as_str(self) -> &'static str {
        match self {
            Hive::CurrentUser => "HKCU",
            Hive::LocalMachine => "HKLM",
        }
    }
}

pub trait RegistryReader: Send + Sync {
    fn read_string(&self, hive: Hive, key: &str, value: &str) -> Option<String>;
    fn read_u32(&self, hive: Hive, key: &str, value: &str) -> Option<u32>;
    /// Immediate child key names, without the parent path.
    fn subkeys(&self, hive: Hive, key: &str) -> Vec<String>;
}

/// A registry that contains nothing. Used on non-Windows builds and as a base
/// for tests that only care about the filesystem.
#[derive(Default)]
pub struct EmptyRegistry;

impl RegistryReader for EmptyRegistry {
    fn read_string(&self, _: Hive, _: &str, _: &str) -> Option<String> {
        None
    }
    fn read_u32(&self, _: Hive, _: &str, _: &str) -> Option<u32> {
        None
    }
    fn subkeys(&self, _: Hive, _: &str) -> Vec<String> {
        Vec::new()
    }
}

/// An in-memory registry for tests. Keys are compared case-insensitively and
/// with both slash directions accepted, because the real registry is forgiving
/// about both and adapters should not have to care.
#[derive(Default, Clone)]
pub struct FakeRegistry {
    values: BTreeMap<(Hive, String, String), String>,
}

fn norm(key: &str) -> String {
    key.replace('/', "\\").trim_matches('\\').to_lowercase()
}

impl FakeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, hive: Hive, key: &str, value: &str, data: &str) -> Self {
        self.values
            .insert((hive, norm(key), value.to_lowercase()), data.to_string());
        self
    }

    /// Registers a key that exists but holds no values, so `subkeys` can find it.
    pub fn with_key(self, hive: Hive, key: &str) -> Self {
        self.with(hive, key, "", "")
    }
}

impl RegistryReader for FakeRegistry {
    fn read_string(&self, hive: Hive, key: &str, value: &str) -> Option<String> {
        self.values
            .get(&(hive, norm(key), value.to_lowercase()))
            .filter(|v| !v.is_empty())
            .cloned()
    }

    fn read_u32(&self, hive: Hive, key: &str, value: &str) -> Option<u32> {
        self.read_string(hive, key, value)?.parse().ok()
    }

    fn subkeys(&self, hive: Hive, key: &str) -> Vec<String> {
        let prefix = format!("{}\\", norm(key));
        let mut out: Vec<String> = self
            .values
            .keys()
            .filter(|(h, k, _)| *h == hive && k.starts_with(&prefix))
            .filter_map(|(_, k, _)| k[prefix.len()..].split('\\').next().map(str::to_string))
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// The Windows folders adapters look in. Held as data so a fixture tree can
/// stand in for all of them at once.
#[derive(Debug, Clone)]
pub struct KnownFolders {
    pub program_data: PathBuf,
    pub app_data: PathBuf,
    pub local_app_data: PathBuf,
    pub program_files: PathBuf,
    pub program_files_x86: PathBuf,
    pub user_profile: PathBuf,
    /// Every fixed drive root, used only when a launcher gives no better hint.
    pub drives: Vec<PathBuf>,
    /// `JAVA_HOME`, when the machine sets it. Held here rather than read from
    /// the process environment so a fixture environment is genuinely sealed —
    /// a test must not find the Java installed on the machine running it.
    pub java_home: Option<PathBuf>,
}

impl KnownFolders {
    /// Points every folder inside one directory, laid out the way Windows lays
    /// them out. Tests build their fixtures against exactly this shape.
    pub fn rooted_at(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            program_data: root.join("ProgramData"),
            app_data: root.join("Users/Test/AppData/Roaming"),
            local_app_data: root.join("Users/Test/AppData/Local"),
            program_files: root.join("Program Files"),
            program_files_x86: root.join("Program Files (x86)"),
            user_profile: root.join("Users/Test"),
            drives: vec![root.to_path_buf()],
            java_home: None,
        }
    }

    #[cfg(windows)]
    pub fn from_system() -> Self {
        let var = |name: &str, fallback: &str| {
            std::env::var(name).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(fallback))
        };
        Self {
            program_data: var("ProgramData", "C:\\ProgramData"),
            app_data: var("APPDATA", "C:\\Users\\Default\\AppData\\Roaming"),
            local_app_data: var("LOCALAPPDATA", "C:\\Users\\Default\\AppData\\Local"),
            program_files: var("ProgramW6432", "C:\\Program Files"),
            program_files_x86: var("ProgramFiles(x86)", "C:\\Program Files (x86)"),
            user_profile: var("USERPROFILE", "C:\\Users\\Default"),
            drives: fixed_drives(),
            java_home: std::env::var("JAVA_HOME").ok().map(PathBuf::from),
        }
    }

    #[cfg(not(windows))]
    pub fn from_system() -> Self {
        // GameHub targets Windows. This exists so the crate builds and tests on
        // Linux CI; it deliberately points at nothing that exists.
        Self::rooted_at("/nonexistent")
    }
}

#[cfg(windows)]
fn fixed_drives() -> Vec<PathBuf> {
    ('C'..='Z')
        .map(|letter| PathBuf::from(format!("{letter}:\\")))
        .filter(|p| p.exists())
        .collect()
}

/// Everything an adapter is allowed to look at.
#[derive(Clone)]
pub struct Env {
    pub folders: KnownFolders,
    pub registry: Arc<dyn RegistryReader>,
}

impl Env {
    pub fn system() -> Self {
        Self {
            folders: KnownFolders::from_system(),
            registry: default_registry(),
        }
    }

    /// A fixture environment: a directory tree plus an optional fake registry.
    pub fn fixture(root: impl AsRef<Path>, registry: FakeRegistry) -> Self {
        Self {
            folders: KnownFolders::rooted_at(root),
            registry: Arc::new(registry),
        }
    }

    pub fn reg_string(&self, hive: Hive, key: &str, value: &str) -> Option<String> {
        self.registry.read_string(hive, key, value)
    }

    /// Reads a value from the 64-bit view and the 32-bit (WOW6432Node) view,
    /// which is where most launcher keys actually live.
    pub fn reg_string_both_views(&self, hive: Hive, key: &str, value: &str) -> Option<String> {
        self.reg_string(hive, key, value).or_else(|| {
            let wow = key
                .strip_prefix("SOFTWARE\\")
                .map(|rest| format!("SOFTWARE\\WOW6432Node\\{rest}"))?;
            self.reg_string(hive, &wow, value)
        })
    }

    pub fn subkeys_both_views(&self, hive: Hive, key: &str) -> Vec<String> {
        let mut keys = self.registry.subkeys(hive, key);
        if let Some(rest) = key.strip_prefix("SOFTWARE\\") {
            keys.extend(
                self.registry
                    .subkeys(hive, &format!("SOFTWARE\\WOW6432Node\\{rest}")),
            );
        }
        keys.sort();
        keys.dedup();
        keys
    }
}

#[cfg(windows)]
fn default_registry() -> Arc<dyn RegistryReader> {
    Arc::new(WindowsRegistry)
}

#[cfg(not(windows))]
fn default_registry() -> Arc<dyn RegistryReader> {
    Arc::new(EmptyRegistry)
}

#[cfg(windows)]
pub struct WindowsRegistry;

#[cfg(windows)]
impl WindowsRegistry {
    fn open(hive: Hive, key: &str) -> Option<winreg::RegKey> {
        use winreg::enums::*;
        let root = winreg::RegKey::predef(match hive {
            Hive::CurrentUser => HKEY_CURRENT_USER,
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
        });
        root.open_subkey_with_flags(key, KEY_READ).ok()
    }
}

#[cfg(windows)]
impl RegistryReader for WindowsRegistry {
    fn read_string(&self, hive: Hive, key: &str, value: &str) -> Option<String> {
        let k = Self::open(hive, key)?;
        let v: String = k.get_value(value).ok()?;
        (!v.is_empty()).then_some(v)
    }

    fn read_u32(&self, hive: Hive, key: &str, value: &str) -> Option<u32> {
        Self::open(hive, key)?.get_value(value).ok()
    }

    fn subkeys(&self, hive: Hive, key: &str) -> Vec<String> {
        match Self::open(hive, key) {
            Some(k) => k.enum_keys().filter_map(std::result::Result::ok).collect(),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_registry_is_case_and_slash_insensitive() {
        let reg = FakeRegistry::new().with(
            Hive::LocalMachine,
            "SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher",
            "InstallDir",
            "C:\\Ubisoft",
        );
        assert_eq!(
            reg.read_string(Hive::LocalMachine, "software/wow6432node/ubisoft/launcher", "installdir"),
            Some("C:\\Ubisoft".into())
        );
        assert_eq!(reg.read_string(Hive::CurrentUser, "SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher", "InstallDir"), None);
    }

    #[test]
    fn subkeys_returns_immediate_children_only() {
        let reg = FakeRegistry::new()
            .with(Hive::LocalMachine, "SOFTWARE\\Ubisoft\\Launcher\\Installs\\720", "InstallDir", "D:\\ac")
            .with(Hive::LocalMachine, "SOFTWARE\\Ubisoft\\Launcher\\Installs\\1234", "InstallDir", "D:\\fc")
            .with(Hive::LocalMachine, "SOFTWARE\\Ubisoft\\Launcher\\Installs\\1234\\Deep", "X", "y");
        let mut keys = reg.subkeys(Hive::LocalMachine, "SOFTWARE\\Ubisoft\\Launcher\\Installs");
        keys.sort();
        assert_eq!(keys, vec!["1234".to_string(), "720".to_string()]);
    }

    #[test]
    fn wow6432node_fallback_finds_32_bit_keys() {
        let env = Env::fixture(
            "/tmp/does-not-matter",
            FakeRegistry::new().with(
                Hive::LocalMachine,
                "SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1207658924",
                "path",
                "D:\\GOG\\Witcher",
            ),
        );
        assert_eq!(
            env.reg_string_both_views(Hive::LocalMachine, "SOFTWARE\\GOG.com\\Games\\1207658924", "path"),
            Some("D:\\GOG\\Witcher".into())
        );
    }
}
