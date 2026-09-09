//! Launcher adapters.
//!
//! Every adapter answers the same four questions: is this launcher installed,
//! where are its libraries, what is installed in them, and how is each game
//! started. Nothing else. Metadata, artwork, favourites and the UI live
//! elsewhere, so adding a launcher means writing one file and registering it.
//!
//! Two rules hold for all of them:
//!
//! * **No hard-coded library path.** A path from the registry or a manifest
//!   first; a well-known default only as a last resort, and only if it exists.
//! * **No invented capability.** If a launcher does not expose playtime, the
//!   field stays `None`; if it cannot be read safely, the adapter reports a
//!   `limitation` string that the Settings page shows the user verbatim.

pub mod battlenet;
pub mod ea;
pub mod epic;
pub mod gog;
pub mod local;
pub mod riot;
pub mod scanner;
pub mod steam;
pub mod ubisoft;
pub mod uninstall;
pub mod xbox;

use gamehub_detect::{Env, Game, GameSource, LauncherStatus};

/// What a single adapter found.
pub struct AdapterScan {
    pub status: LauncherStatus,
    pub games: Vec<Game>,
}

impl AdapterScan {
    pub fn missing(source: GameSource) -> Self {
        Self {
            status: LauncherStatus {
                source,
                detected: false,
                install_dir: None,
                game_count: 0,
                limitation: None,
            },
            games: Vec::new(),
        }
    }

    pub fn found(source: GameSource, install_dir: Option<String>, games: Vec<Game>) -> Self {
        Self {
            status: LauncherStatus {
                source,
                detected: true,
                install_dir,
                game_count: games.len(),
                limitation: None,
            },
            games,
        }
    }

    pub fn with_limitation(mut self, note: impl Into<String>) -> Self {
        self.status.limitation = Some(note.into());
        self
    }
}

pub trait LauncherAdapter: Send + Sync {
    fn source(&self) -> GameSource;
    /// Never fails: a launcher that is not installed is a normal outcome, and a
    /// launcher that is installed but unreadable reports a limitation instead
    /// of taking the whole scan down with it.
    fn scan(&self, env: &Env) -> AdapterScan;
}

/// Every adapter, in the order the onboarding screen lists them.
pub fn all_adapters() -> Vec<Box<dyn LauncherAdapter>> {
    vec![
        Box::new(steam::SteamAdapter),
        Box::new(epic::EpicAdapter),
        Box::new(xbox::XboxAdapter),
        Box::new(ea::EaAdapter),
        Box::new(ubisoft::UbisoftAdapter),
        Box::new(battlenet::BattleNetAdapter),
        Box::new(gog::GogAdapter),
        Box::new(riot::RiotAdapter),
    ]
}

/// Shared helper: the first path in `candidates` that exists on disk.
pub(crate) fn first_existing<I, P>(candidates: I) -> Option<std::path::PathBuf>
where
    I: IntoIterator<Item = P>,
    P: Into<std::path::PathBuf>,
{
    candidates.into_iter().map(Into::into).find(|p| p.exists())
}

/// Directory size, capped so a scan cannot be made slow by a pathological tree.
pub(crate) fn directory_size(path: &std::path::Path, max_entries: usize) -> Option<u64> {
    let mut total = 0u64;
    let mut seen = 0usize;
    for entry in walkdir::WalkDir::new(path).max_depth(6).into_iter().flatten() {
        seen += 1;
        if seen > max_entries {
            return None;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total += meta.len();
            }
        }
    }
    Some(total)
}
