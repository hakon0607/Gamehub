//! "Start with Windows", written the way Windows expects: a value under the
//! current user's Run key. No scheduled task, no service, nothing that needs
//! administrator rights, and removing it is a single delete.

#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "GameHub";

#[cfg(windows)]
pub fn set(enabled: bool) -> std::io::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(RUN_KEY)?;
    if enabled {
        let exe = std::env::current_exe()?;
        // `--tray` starts minimised, so logging in does not throw a window in
        // the user's face.
        key.set_value(VALUE_NAME, &format!("\"{}\" --tray", exe.display()))
    } else {
        match key.delete_value(VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(not(windows))]
pub fn set(enabled: bool) -> std::io::Result<()> {
    let _ = enabled;
    Ok(())
}
