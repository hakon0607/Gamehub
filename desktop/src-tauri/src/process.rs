//! Is this game running — and is it being played?
//!
//! GameHub does not hook into games or read their memory. It compares the
//! executable paths of running processes against the install folders it already
//! knows, which is enough to grey out a Play button and to record play time,
//! and requires no special privileges.
//!
//! "Being played" is two cheap questions Windows answers directly: which
//! process owns the window in front (`GetForegroundWindow`), and how long ago
//! the keyboard or mouse was last touched (`GetLastInputInfo`). A game whose
//! window is in front while the player is at the keyboard is being played;
//! anything else — alt-tabbed to Discord, a launcher idling in the tray, a
//! long walk away from the PC — is not, and its clock pauses.

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
        self.observe(library, None).into_iter().map(|(id, _)| id).collect()
    }

    /// Every running game, and whether it is being played right now: its
    /// process owns the window in front and the player has touched the
    /// keyboard or mouse within `idle_limit`. `None` means "in front counts,
    /// idle does not matter".
    pub fn observe(&mut self, library: &[Game], idle_limit: Option<std::time::Duration>) -> Vec<(String, bool)> {
        self.system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let paths: Vec<std::path::PathBuf> = self
            .system
            .processes()
            .values()
            .filter_map(|p| p.exe().map(Path::to_path_buf))
            .collect();

        // The executable behind the window in front, if it is a process we
        // can see.
        let front = os::foreground_pid()
            .and_then(|pid| self.system.process(sysinfo::Pid::from_u32(pid)))
            .and_then(|p| p.exe().map(Path::to_path_buf));
        let awake = match (idle_limit, os::idle_time()) {
            (Some(limit), Some(idle)) => idle <= limit,
            _ => true,
        };

        let mut seen: Vec<(String, bool)> = Vec::new();
        let mut ids = HashSet::new();
        for game in library {
            let Some(dir) = &game.install_dir else { continue };
            let dir = Path::new(dir);
            if !paths.iter().any(|exe| safepath::is_within(dir, exe)) {
                continue;
            }
            if !ids.insert(game.id.clone()) {
                continue;
            }
            // Off Windows nothing can say what is in front, so every running
            // game counts — the behaviour before active tracking existed.
            let in_front = if cfg!(windows) {
                front.as_ref().map(|exe| safepath::is_within(dir, exe)).unwrap_or(false)
            } else {
                true
            };
            seen.push((game.id.clone(), in_front && awake));
        }
        seen.sort();
        seen
    }
}

#[cfg(windows)]
mod os {
    use std::time::Duration;
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    /// The process that owns the window in front, if any.
    pub fn foreground_pid() -> Option<u32> {
        // SAFETY: plain Win32 calls with a valid out-pointer.
        unsafe {
            let window = GetForegroundWindow();
            if window.is_null() {
                return None;
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(window, &mut pid);
            (pid != 0).then_some(pid)
        }
    }

    /// Time since the keyboard or mouse was last touched.
    pub fn idle_time() -> Option<Duration> {
        // SAFETY: the struct is sized as the API requires.
        unsafe {
            let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
            if GetLastInputInfo(&mut info) == 0 {
                return None;
            }
            let now = GetTickCount();
            Some(Duration::from_millis(now.wrapping_sub(info.dwTime) as u64))
        }
    }
}

#[cfg(not(windows))]
mod os {
    use std::time::Duration;
    // Off Windows there is no window in front to ask about; every running
    // game counts as played, which is the old behaviour.
    pub fn foreground_pid() -> Option<u32> {
        None
    }
    pub fn idle_time() -> Option<Duration> {
        None
    }
}

impl Default for ProcessWatch {
    fn default() -> Self {
        Self::new()
    }
}
