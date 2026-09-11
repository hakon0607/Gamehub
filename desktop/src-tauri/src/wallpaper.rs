//! Wallpapers: the desktop background and the lock screen picture.
//!
//! What Windows actually allows a normal program to do, and what this does:
//!
//! * **Desktop.** `IDesktopWallpaper` (Windows 8 and later) sets the
//!   background for every monitor at once or for one monitor by its device
//!   path, and reads back what each monitor shows now. That is the same
//!   interface the Settings app uses, so the change is immediate and Windows
//!   remembers it across restarts by itself.
//! * **Lock screen.** Windows exposes exactly one supported way for a program
//!   running as the user to change the lock screen picture:
//!   `Windows.System.UserProfile.LockScreen.SetImageFileAsync`. It changes the
//!   picture for the signed-in user (not the sign-in screen before anyone has
//!   signed in, which is a machine setting needing administrator rights) and
//!   switches the lock screen from Windows Spotlight to "Picture". Windows
//!   keeps its own copy of the file. There is no supported way to read the
//!   current picture back, so the card shows what GameHub last sent. A group
//!   policy that locks the lock screen makes the call fail, and the error is
//!   shown as it is rather than pretending.
//!
//! The picture the user picked is copied into GameHub's own data folder,
//! like the app background is, so it keeps working after the original is
//! moved, survives a backup and restore, and can be shown by the web view,
//! which is only allowed to read files GameHub owns.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// What is stored in settings: the copies GameHub made, by where they go.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct WallpaperSettings {
    /// The desktop background for every monitor, or empty.
    pub desktop: String,
    /// The lock screen picture last sent to Windows, or empty.
    pub lock: String,
    /// Per-monitor overrides, keyed by the monitor's device path.
    pub monitors: std::collections::BTreeMap<String, String>,
}

/// One chosen picture, described for the card that shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Picture {
    pub path: String,
    pub file_name: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

/// One monitor, as Windows lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: String,
    pub index: u32,
    pub width: u32,
    pub height: u32,
    /// The background this monitor shows right now, straight from Windows.
    pub current: String,
    /// GameHub's own choice for this monitor, when there is one.
    pub chosen: Option<Picture>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperStatus {
    /// Desktop backgrounds can be set on this machine.
    pub supported: bool,
    /// The lock screen API exists on this machine.
    pub lock_supported: bool,
    pub desktop: Option<Picture>,
    /// True when Windows reports GameHub's desktop choice as the background
    /// in use (on every monitor without its own override).
    pub desktop_active: bool,
    pub lock: Option<Picture>,
    pub monitors: Vec<Monitor>,
    /// Where the copies live, for "open folder".
    pub folder: String,
}

/// Where a picture goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Desktop,
    Lock,
}

impl Target {
    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "desktop" => Ok(Target::Desktop),
            "lock" => Ok(Target::Lock),
            _ => Err(crate::msg::plain("wallpaper_bad_target")),
        }
    }
}

const EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "bmp"];

/// Reads the picture's size and checks it really is an image of a kind
/// Windows will show. Format is judged by content, not by name.
pub fn describe(path: &Path) -> Result<Picture, String> {
    let meta = std::fs::metadata(path).map_err(|e| crate::msg::code("file_open", &[&e.to_string()]))?;
    if !meta.is_file() {
        return Err(crate::msg::plain("image_not_file"));
    }
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|e| EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| crate::msg::plain("wallpaper_bad_format"))?;
    let mut head = [0u8; 16];
    {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(|e| crate::msg::code("file_open", &[&e.to_string()]))?;
        let _ = file.read(&mut head);
    }
    let looks_like = crate::artwork::sniff_image(&head).ok_or_else(|| crate::msg::plain("wallpaper_bad_format"))?;
    if looks_like == "image/gif" {
        return Err(crate::msg::plain("wallpaper_bad_format"));
    }
    let (width, height) =
        image::image_dimensions(path).map_err(|e| crate::msg::code("wallpaper_unreadable", &[&e.to_string()]))?;
    let _ = extension;
    Ok(Picture {
        path: path.display().to_string(),
        file_name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        width,
        height,
        bytes: meta.len(),
    })
}

/// The name a copy gets inside GameHub's wallpaper folder: where it goes,
/// when it was chosen, and the original extension so Windows knows the type.
pub fn copy_name(target: Target, monitor_index: Option<u32>, original: &Path, stamp: &str) -> String {
    let extension = original
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "jpg".into());
    let slot = match (target, monitor_index) {
        (Target::Lock, _) => "lock".to_string(),
        (Target::Desktop, None) => "desktop".to_string(),
        (Target::Desktop, Some(index)) => format!("monitor-{index}"),
    };
    format!("{slot}-{}.{extension}", gamehub_detect::backup::slug(stamp))
}

/// Copies the picked file into GameHub's folder — unless it is already one
/// of GameHub's copies, which is what "use the same picture on both" sends.
/// Older copies for the same slot are deleted; only one is ever in use.
pub fn adopt(folder: &Path, target: Target, monitor_index: Option<u32>, source: &Path) -> Result<PathBuf, String> {
    if source.starts_with(folder) && source.is_file() {
        let name = copy_name(target, monitor_index, source, &gamehub_detect::now_iso8601());
        let copy = gamehub_detect::safepath::join_within(folder, &name).map_err(|e| e.to_string())?;
        std::fs::copy(source, &copy).map_err(|e| crate::msg::code("wallpaper_copy", &[&e.to_string()]))?;
        prune(folder, target, monitor_index, &copy);
        return Ok(copy);
    }
    std::fs::create_dir_all(folder).map_err(|e| crate::msg::code("wallpaper_copy", &[&e.to_string()]))?;
    let name = copy_name(target, monitor_index, source, &gamehub_detect::now_iso8601());
    let copy = gamehub_detect::safepath::join_within(folder, &name).map_err(|e| e.to_string())?;
    std::fs::copy(source, &copy).map_err(|e| crate::msg::code("wallpaper_copy", &[&e.to_string()]))?;
    prune(folder, target, monitor_index, &copy);
    Ok(copy)
}

fn prune(folder: &Path, target: Target, monitor_index: Option<u32>, keep: &Path) {
    let prefix = match (target, monitor_index) {
        (Target::Lock, _) => "lock-".to_string(),
        (Target::Desktop, None) => "desktop-".to_string(),
        (Target::Desktop, Some(index)) => format!("monitor-{index}-"),
    };
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&prefix) && path != keep {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Windows' own default pictures, for "back to the Windows default". They
/// ship with every Windows 10 and 11; if a machine lacks them the error says
/// so instead of guessing.
pub fn windows_default(target: Target) -> Result<PathBuf, String> {
    let root = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let (folder, preferred) = match target {
        Target::Desktop => (root.join("Web").join("Wallpaper").join("Windows"), "img0.jpg"),
        Target::Lock => (root.join("Web").join("Screen"), "img100.jpg"),
    };
    let first_choice = folder.join(preferred);
    if first_choice.is_file() {
        return Ok(first_choice);
    }
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("png")).unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    candidates.sort();
    candidates.into_iter().next().ok_or_else(|| crate::msg::plain("wallpaper_no_default"))
}

/// The full picture for the cards, reading current state from Windows.
pub fn status(state: &AppState) -> WallpaperStatus {
    let settings = state.settings();
    let chosen = settings.wallpapers.clone();
    let folder = state.paths.wallpapers();

    let picture = |path: &str| if path.is_empty() { None } else { describe(Path::new(path)).ok() };
    let desktop = picture(&chosen.desktop);
    let lock = picture(&chosen.lock);

    let monitors: Vec<Monitor> = os::monitors()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, m)| Monitor {
            chosen: chosen.monitors.get(&m.id).and_then(|p| picture(p)),
            id: m.id,
            index: index as u32,
            width: m.width,
            height: m.height,
            current: m.current,
        })
        .collect();

    // Active when every monitor without its own override shows our file.
    let desktop_active = desktop.is_some()
        && !monitors.is_empty()
        && monitors.iter().all(|m| {
            let wanted = m.chosen.as_ref().map(|p| p.path.as_str()).unwrap_or(chosen.desktop.as_str());
            same_file(&m.current, wanted)
        });

    WallpaperStatus {
        supported: os::supported(),
        lock_supported: os::lock_supported(),
        desktop,
        desktop_active,
        lock,
        monitors,
        folder: folder.display().to_string(),
    }
}

fn same_file(a: &str, b: &str) -> bool {
    !a.is_empty() && a.eq_ignore_ascii_case(b)
}

/// Sets a picture: validates it, copies it, hands it to Windows, and only
/// then remembers it — so a refusal from Windows leaves settings untouched.
pub fn apply(state: &AppState, target: Target, source: &Path, monitor: Option<&str>) -> Result<(), String> {
    describe(source)?;
    let folder = state.paths.wallpapers();
    let monitor_index = match monitor {
        Some(id) => Some(monitor_index_of(id)?),
        None => None,
    };
    let copy = adopt(&folder, target, monitor_index, source)?;
    describe(&copy)?;

    match target {
        Target::Desktop => os::set_desktop(&copy, monitor)?,
        Target::Lock => os::set_lock(&copy)?,
    }

    {
        let mut inner = state.inner.lock();
        let wallpapers = &mut inner.settings.wallpapers;
        match (target, monitor) {
            (Target::Lock, _) => wallpapers.lock = copy.display().to_string(),
            (Target::Desktop, None) => {
                wallpapers.desktop = copy.display().to_string();
                // "All monitors" replaces every per-monitor choice.
                wallpapers.monitors.clear();
            }
            (Target::Desktop, Some(id)) => {
                wallpapers.monitors.insert(id.to_string(), copy.display().to_string());
            }
        }
    }
    state.persist_settings();
    Ok(())
}

/// Forgets GameHub's choice. Windows keeps showing whatever it shows now;
/// nothing is changed on screen — except a monitor override, which goes back
/// to the shared desktop picture when there is one, so the monitor is not
/// left pointing at a copy that is about to be deleted.
pub fn forget(state: &AppState, target: Target, monitor: Option<&str>) -> Result<(), String> {
    let folder = state.paths.wallpapers();
    if let (Target::Desktop, Some(id)) = (target, monitor) {
        let shared = state.settings().wallpapers.desktop;
        if !shared.is_empty() && Path::new(&shared).is_file() {
            os::set_desktop(Path::new(&shared), Some(id))?;
        }
    }
    {
        let mut inner = state.inner.lock();
        let wallpapers = &mut inner.settings.wallpapers;
        let gone = match (target, monitor) {
            (Target::Lock, _) => std::mem::take(&mut wallpapers.lock),
            (Target::Desktop, None) => std::mem::take(&mut wallpapers.desktop),
            (Target::Desktop, Some(id)) => wallpapers.monitors.remove(id).unwrap_or_default(),
        };
        let gone = PathBuf::from(gone);
        if gone.starts_with(&folder) && gone.is_file() {
            let _ = std::fs::remove_file(gone);
        }
    }
    state.persist_settings();
    Ok(())
}

/// Puts Windows' own default picture back and forgets GameHub's choice.
pub fn restore_default(state: &AppState, target: Target, monitor: Option<&str>) -> Result<(), String> {
    let default = windows_default(target)?;
    match target {
        Target::Desktop => os::set_desktop(&default, monitor)?,
        Target::Lock => os::set_lock(&default)?,
    }
    forget(state, target, monitor)
}

fn monitor_index_of(id: &str) -> Result<u32, String> {
    os::monitors()?
        .iter()
        .position(|m| m.id == id)
        .map(|i| i as u32)
        .ok_or_else(|| crate::msg::plain("wallpaper_no_monitor"))
}

/// One monitor as Windows reports it.
#[derive(Debug, Clone)]
pub struct OsMonitor {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub current: String,
}

#[cfg(windows)]
pub mod os {
    use super::OsMonitor;
    use std::path::Path;
    use windows::core::{HSTRING, PCWSTR, PWSTR};
    use windows::Storage::StorageFile;
    use windows::System::UserProfile::LockScreen;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper, DWPOS_FILL};

    /// COM for the current thread, balanced on drop. These calls run on a
    /// worker thread, never the UI thread.
    struct Com {
        initialised: bool,
    }

    impl Com {
        fn new() -> Self {
            // SAFETY: plain COM initialisation with no reserved pointer.
            let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            Com { initialised: result.is_ok() }
        }
    }

    impl Drop for Com {
        fn drop(&mut self) {
            if self.initialised {
                // SAFETY: paired with the successful CoInitializeEx above.
                unsafe { CoUninitialize() };
            }
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn desktop() -> Result<IDesktopWallpaper, String> {
        // SAFETY: standard in-process COM activation of a system class.
        unsafe { CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL) }
            .map_err(|e| crate::msg::code("wallpaper_com", &[&e.to_string()]))
    }

    /// Takes ownership of a COM-allocated string and frees it.
    fn take(text: PWSTR) -> String {
        if text.is_null() {
            return String::new();
        }
        // SAFETY: the pointer came from IDesktopWallpaper, which allocates
        // with CoTaskMemAlloc and hands ownership to the caller.
        unsafe {
            let owned = text.to_string().unwrap_or_default();
            CoTaskMemFree(Some(text.0 as *const core::ffi::c_void));
            owned
        }
    }

    pub fn supported() -> bool {
        let _com = Com::new();
        desktop().is_ok()
    }

    pub fn lock_supported() -> bool {
        // The WinRT class exists on every Windows 10 and 11; whether a call
        // succeeds depends on policy, which only the call itself can tell.
        true
    }

    pub fn monitors() -> Result<Vec<OsMonitor>, String> {
        let _com = Com::new();
        let wallpaper = desktop()?;
        // SAFETY: every call below is a documented IDesktopWallpaper method
        // with the arguments it expects; returned strings are freed by `take`.
        unsafe {
            let count = wallpaper
                .GetMonitorDevicePathCount()
                .map_err(|e| crate::msg::code("wallpaper_com", &[&e.to_string()]))?;
            let mut found = Vec::new();
            for index in 0..count {
                let Ok(id) = wallpaper.GetMonitorDevicePathAt(index) else { continue };
                let id_wide = wide(&id.to_string().unwrap_or_default());
                let id_text = take(id);
                if id_text.is_empty() {
                    continue;
                }
                // A monitor Windows remembers but that is not attached now has
                // no rectangle; it is not something the user can see.
                let Ok(rect) = wallpaper.GetMonitorRECT(PCWSTR(id_wide.as_ptr())) else { continue };
                let current = wallpaper.GetWallpaper(PCWSTR(id_wide.as_ptr())).map(take).unwrap_or_default();
                found.push(OsMonitor {
                    id: id_text,
                    width: (rect.right - rect.left).max(0) as u32,
                    height: (rect.bottom - rect.top).max(0) as u32,
                    current,
                });
            }
            Ok(found)
        }
    }

    pub fn set_desktop(path: &Path, monitor: Option<&str>) -> Result<(), String> {
        let _com = Com::new();
        let wallpaper = desktop()?;
        let path_wide = wide(&path.display().to_string());
        // SAFETY: documented IDesktopWallpaper calls; the wide strings live
        // for the duration of each call.
        unsafe {
            let _ = wallpaper.SetPosition(DWPOS_FILL);
            let result = match monitor {
                Some(id) => {
                    let id_wide = wide(id);
                    wallpaper.SetWallpaper(PCWSTR(id_wide.as_ptr()), PCWSTR(path_wide.as_ptr()))
                }
                // A null monitor id means every monitor.
                None => wallpaper.SetWallpaper(PCWSTR::null(), PCWSTR(path_wide.as_ptr())),
            };
            result.map_err(|e| crate::msg::code("wallpaper_desktop_failed", &[&e.to_string()]))
        }
    }

    pub fn set_lock(path: &Path) -> Result<(), String> {
        let _com = Com::new();
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.display().to_string()))
            .and_then(|op| op.get())
            .map_err(|e| crate::msg::code("wallpaper_lock_failed", &[&e.to_string()]))?;
        LockScreen::SetImageFileAsync(&file)
            .and_then(|op| op.get())
            .map_err(|e| crate::msg::code("wallpaper_lock_failed", &[&e.to_string()]))
    }
}

#[cfg(not(windows))]
pub mod os {
    use super::OsMonitor;
    use std::path::Path;

    // GameHub is a Windows application. This exists so the crate builds and
    // is testable elsewhere, and says so instead of pretending.
    pub fn supported() -> bool {
        false
    }
    pub fn lock_supported() -> bool {
        false
    }
    pub fn monitors() -> Result<Vec<OsMonitor>, String> {
        Ok(Vec::new())
    }
    pub fn set_desktop(_path: &Path, _monitor: Option<&str>) -> Result<(), String> {
        Err(crate::msg::plain("wallpaper_windows_only"))
    }
    pub fn set_lock(_path: &Path) -> Result<(), String> {
        Err(crate::msg::plain("wallpaper_windows_only"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(dir: &Path, name: &str, width: u32, height: u32) -> PathBuf {
        let path = dir.join(name);
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]));
        image.save(&path).unwrap();
        path
    }

    #[test]
    fn a_real_picture_is_described_with_its_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = png(dir.path(), "sky.png", 64, 32);
        let picture = describe(&path).unwrap();
        assert_eq!((picture.width, picture.height), (64, 32));
        assert_eq!(picture.file_name, "sky.png");
        assert!(picture.bytes > 0);
    }

    #[test]
    fn a_renamed_text_file_is_refused_by_content_not_name() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("not-really.jpg");
        std::fs::write(&fake, b"hello, this is text").unwrap();
        assert_eq!(describe(&fake).unwrap_err(), "@wallpaper_bad_format");
    }

    #[test]
    fn unsupported_extensions_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = png(dir.path(), "sky.tiff", 4, 4);
        assert_eq!(describe(&path).unwrap_err(), "@wallpaper_bad_format");
    }

    #[test]
    fn adopting_copies_into_the_folder_and_keeps_only_the_newest_per_slot() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("wallpapers");
        let first = png(dir.path(), "a.png", 8, 8);
        let second = png(dir.path(), "b.png", 8, 8);

        let copy_a = adopt(&folder, Target::Desktop, None, &first).unwrap();
        assert!(copy_a.starts_with(&folder));
        assert!(copy_a.file_name().unwrap().to_string_lossy().starts_with("desktop-"));

        std::thread::sleep(std::time::Duration::from_millis(1100));
        let copy_b = adopt(&folder, Target::Desktop, None, &second).unwrap();
        assert!(!copy_a.exists(), "the older desktop copy is pruned");
        assert!(copy_b.exists());

        // The lock slot is separate and untouched by desktop changes.
        let lock = adopt(&folder, Target::Lock, None, &first).unwrap();
        assert!(lock.file_name().unwrap().to_string_lossy().starts_with("lock-"));
        assert!(copy_b.exists());
    }

    #[test]
    fn using_the_same_picture_on_both_copies_gamehubs_own_file() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("wallpapers");
        let source = png(dir.path(), "a.png", 8, 8);
        let desktop = adopt(&folder, Target::Desktop, None, &source).unwrap();
        let lock = adopt(&folder, Target::Lock, None, &desktop).unwrap();
        assert!(lock.exists() && desktop.exists());
        assert_ne!(lock, desktop);
    }

    #[test]
    fn copy_names_say_where_the_picture_goes() {
        let original = Path::new(r"C:\Pictures\Sky.JPG");
        assert!(copy_name(Target::Desktop, None, original, "2026-09-10T12:00:00Z").starts_with("desktop-"));
        assert!(copy_name(Target::Desktop, Some(1), original, "2026-09-10T12:00:00Z").starts_with("monitor-1-"));
        assert!(copy_name(Target::Lock, None, original, "2026-09-10T12:00:00Z").ends_with(".jpg"));
    }

    #[test]
    fn targets_parse_and_reject() {
        assert_eq!(Target::parse("desktop").unwrap(), Target::Desktop);
        assert_eq!(Target::parse("lock").unwrap(), Target::Lock);
        assert_eq!(Target::parse("fridge").unwrap_err(), "@wallpaper_bad_target");
    }

    #[cfg(not(windows))]
    #[test]
    fn off_windows_says_so_instead_of_pretending() {
        assert!(!os::supported());
        assert_eq!(os::set_desktop(Path::new("x.png"), None).unwrap_err(), "@wallpaper_windows_only");
    }
}
