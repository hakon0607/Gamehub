//! Freezing a running game.
//!
//! Windows can suspend a whole process — every thread at once — through
//! `NtSuspendProcess`, and resume it the same way. Nothing is injected into the
//! game and nothing about it is changed; it simply gets no CPU time until it is
//! resumed, and continues from the exact instruction it was on. A cutscene, a
//! dialogue with a timer, a boss fight — all stop where they stand.
//!
//! `NtSuspendProcess` is not in the documented API, but it has been in ntdll
//! since Windows XP and is what Process Explorer and every "suspend" button in
//! every task manager uses. It is resolved at run time rather than linked, so
//! the app still starts on a machine where it is somehow missing and says so.
//!
//! Everything platform-neutral — the index of freeze points, the save copies —
//! lives in `gamehub_detect::freeze` and is tested on Linux. This file is the
//! thin Windows layer and a Linux stand-in that reports "not supported" so the
//! crate compiles and tests everywhere.

use std::path::Path;

use gamehub_detect::safepath;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// The process ids of a game: every running executable inside its install
/// folder. A launcher's own helper processes live elsewhere and are not
/// touched; the game's own child processes (anti-cheat, a second engine
/// process) are inside the folder and are, which is what makes the freeze
/// clean — freezing the main process alone leaves its helpers spinning.
pub fn pids_for(install_dir: &Path) -> Vec<u32> {
    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::Always)),
    );
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut pids: Vec<u32> = system
        .processes()
        .iter()
        .filter(|(_, process)| process.exe().map(|exe| safepath::is_within(install_dir, exe)).unwrap_or(false))
        .map(|(pid, _)| pid.as_u32())
        .collect();
    pids.sort_unstable();
    pids
}

/// Whether a process still exists.
pub fn is_alive(pid: u32) -> bool {
    let mut system = System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]), true);
    system.process(sysinfo::Pid::from_u32(pid)).is_some()
}

/// Suspends every listed process. Fails on the first that cannot be, after
/// resuming the ones already done, so a half-frozen game is never left behind.
pub fn suspend_all(pids: &[u32]) -> Result<(), String> {
    let mut done: Vec<u32> = Vec::new();
    for pid in pids {
        if let Err(error) = platform::suspend(*pid) {
            for undo in &done {
                let _ = platform::resume(*undo);
            }
            return Err(error);
        }
        done.push(*pid);
    }
    Ok(())
}

/// Resumes every listed process. A process that has since exited is skipped,
/// not an error — the point is that the ones still there get going again.
pub fn resume_all(pids: &[u32]) -> Result<usize, String> {
    let mut resumed = 0;
    let mut first_error: Option<String> = None;
    for pid in pids {
        if !is_alive(*pid) {
            continue;
        }
        match platform::resume(*pid) {
            Ok(()) => resumed += 1,
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    match first_error {
        Some(error) if resumed == 0 => Err(error),
        _ => Ok(resumed),
    }
}

/// Whether this build can freeze at all.
pub fn supported() -> bool {
    platform::supported()
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    type NtProcessFn = unsafe extern "system" fn(Handle) -> i32;

    // Declared here rather than through windows-sys so the four calls this
    // needs are exactly the four it links, with no feature flags to get wrong.
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        fn GetModuleHandleW(module_name: *const u16) -> Handle;
        fn GetProcAddress(module: Handle, proc_name: *const u8) -> Option<unsafe extern "system" fn() -> isize>;
        fn GetLastError() -> u32;
    }

    const PROCESS_SUSPEND_RESUME: u32 = 0x0800;

    fn ntdll_function(name: &[u8]) -> Option<NtProcessFn> {
        // "ntdll.dll" as UTF-16, NUL-terminated.
        let module_name: Vec<u16> = "ntdll.dll".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let module = GetModuleHandleW(module_name.as_ptr());
            if module.is_null() {
                return None;
            }
            let address = GetProcAddress(module, name.as_ptr())?;
            Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, NtProcessFn>(address))
        }
    }

    fn call(pid: u32, name: &[u8], verb: &str) -> Result<(), String> {
        let Some(function) = ntdll_function(name) else {
            let _ = verb;
            return Err(crate::msg::plain("nt_missing"));
        };
        unsafe {
            let handle = OpenProcess(PROCESS_SUSPEND_RESUME, 0, pid);
            if handle.is_null() {
                let code = GetLastError();
                return Err(match code {
                    5 => crate::msg::code("freeze_denied", &[&pid.to_string()]),
                    87 => crate::msg::code("process_gone", &[&pid.to_string()]),
                    other => crate::msg::code("process_open", &[&pid.to_string(), &other.to_string()]),
                });
            }
            let status = function(handle);
            CloseHandle(handle);
            if status < 0 {
                return Err(crate::msg::code("nt_failed", &[&pid.to_string(), &format!("{status:#x}")]));
            }
        }
        Ok(())
    }

    pub fn suspend(pid: u32) -> Result<(), String> {
        call(pid, b"NtSuspendProcess\0", "frysing")
    }

    pub fn resume(pid: u32) -> Result<(), String> {
        call(pid, b"NtResumeProcess\0", "gjenoppta")
    }

    pub fn supported() -> bool {
        ntdll_function(b"NtSuspendProcess\0").is_some() && ntdll_function(b"NtResumeProcess\0").is_some()
    }
}

#[cfg(not(windows))]
mod platform {
    // GameHub targets Windows. On anything else freezing is reported as
    // unsupported rather than faked with SIGSTOP, which would behave
    // differently enough to mislead a test.
    pub fn suspend(_pid: u32) -> Result<(), String> {
        Err(crate::msg::plain("freeze_windows_only"))
    }

    pub fn resume(_pid: u32) -> Result<(), String> {
        Err(crate::msg::plain("freeze_windows_only"))
    }

    pub fn supported() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_test_process_is_alive_and_a_silly_pid_is_not() {
        assert!(is_alive(std::process::id()));
        assert!(!is_alive(u32::MAX - 7));
    }

    #[test]
    fn pids_are_found_by_install_folder() {
        // This test binary is a "game" installed in its own folder.
        let exe = std::env::current_exe().unwrap();
        let folder = exe.parent().unwrap();
        let found = pids_for(folder);
        assert!(found.contains(&std::process::id()), "{found:?} should contain {}", std::process::id());
        assert!(pids_for(Path::new("/definitely/not/a/folder")).is_empty());
    }

    #[test]
    fn resuming_a_process_that_is_gone_is_not_an_error() {
        // The pid does not exist, so there is nothing to resume — and the user
        // must not be told the resume "failed" for a game that already closed.
        assert_eq!(resume_all(&[u32::MAX - 7]), Ok(0));
    }

    #[cfg(not(windows))]
    #[test]
    fn suspending_off_windows_says_so_instead_of_pretending() {
        assert!(!supported());
        let error = suspend_all(&[std::process::id()]).unwrap_err();
        assert_eq!(error, "@freeze_windows_only", "a message code the interface translates");
    }
}
