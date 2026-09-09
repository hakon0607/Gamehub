//! Is this game running?
//!
//! GameHub does not hook into games or read their memory. It compares the
//! executable paths of running processes against the install folders it already
//! knows, which is enough to grey out a Play button and to record play time,
//! and requires no special privileges.

use std::collections::HashSet;
use std::path::Path;

use gamehub_detect::{model::Game, safepath};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

pub struct ProcessWatch {
    system: System,
}

impl ProcessWatch {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::new().with_processes(ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::Always)),
            ),
        }
    }

    /// The ids of games whose install folder contains a running executable.
    pub fn running_games(&mut self, library: &[Game]) -> Vec<String> {
        self.system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let paths: Vec<std::path::PathBuf> = self
            .system
            .processes()
            .values()
            .filter_map(|p| p.exe().map(Path::to_path_buf))
            .collect();

        let mut running = HashSet::new();
        for game in library {
            let Some(dir) = &game.install_dir else { continue };
            let dir = Path::new(dir);
            if paths.iter().any(|exe| safepath::is_within(dir, exe)) {
                running.insert(game.id.clone());
            }
        }

        let mut out: Vec<String> = running.into_iter().collect();
        out.sort();
        out
    }
}

impl Default for ProcessWatch {
    fn default() -> Self {
        Self::new()
    }
}
