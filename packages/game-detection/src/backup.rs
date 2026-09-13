//! Keeping the user's data across updates.
//!
//! GameHub's data — the library, which games were added by hand, which were
//! hidden, the covers, the screenshot index, the shortcuts — is small JSON in
//! the app data folder. Losing it means doing all that work again, so this
//! module exists to make that outcome very hard to reach.
//!
//! Three separate protections, because they fail in different ways:
//!
//! 1. **A snapshot before a new version writes anything.** The first launch
//!    after an update copies the whole state aside first. If the new version
//!    reads something wrong, the old data is still sitting there untouched.
//! 2. **Quarantine instead of reset.** A file that will not parse is renamed,
//!    never overwritten. The old behaviour — fall back to an empty default and
//!    then save over the top — turned one bad byte into a wiped library.
//! 3. **Automatic recovery.** When a file is quarantined, the newest snapshot
//!    that does parse is put back in its place.
//!
//! Everything here works on plain paths so it can be tested on any platform.

use std::path::{Path, PathBuf};

/// A saved copy of the data folder's state files.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// Folder name, which is also the sort key: "2026-08-30T22-14-05-update-0.2.0".
    pub id: String,
    /// Why it was taken, shown to the user.
    pub reason: String,
    /// The app version that was running when it was taken.
    pub app_version: String,
    pub taken_at: String,
    pub files: Vec<String>,
    pub bytes: u64,
}

/// The state files worth snapshotting. Screenshots and clips themselves are not
/// copied — they can be gigabytes, and they are never rewritten by an update.
/// Their *index* files are here, which is what actually holds the metadata.
pub const STATE_FILES: &[&str] = &[
    "library.json",
    "settings.json",
    "activity.json",
    "screenshots.json",
    "clips.json",
    "clipboard.json",
    "shortcuts.json",
];

/// Where snapshots live: a folder *beside* the app data folder, not inside it.
///
/// This matters more than it looks. A Tauri update installs the new version by
/// running the old uninstaller first, and a Windows uninstaller may remove the
/// whole `%APPDATA%\<identifier>` tree. Backups kept inside that tree would be
/// deleted by the very event they exist to protect against. A sibling folder is
/// not something GameHub's installer ever targets, so the snapshots survive
/// even a full uninstall — and are found again by the next install.
pub fn backups_dir(data_dir: &Path) -> PathBuf {
    match (data_dir.parent(), data_dir.file_name()) {
        (Some(parent), Some(name)) => parent.join(format!("{} Backups", name.to_string_lossy())),
        // A path with no parent should not happen, but falling back to a
        // subfolder is far better than panicking on startup.
        _ => data_dir.join("backups"),
    }
}

pub fn version_marker(data_dir: &Path) -> PathBuf {
    data_dir.join("data-version.json")
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionMarker {
    /// The app version that last wrote to this folder.
    pub app_version: String,
    pub updated_at: String,
}

pub fn read_marker(data_dir: &Path) -> Option<VersionMarker> {
    let text = std::fs::read_to_string(version_marker(data_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_marker(data_dir: &Path, app_version: &str, now: &str) -> std::io::Result<()> {
    let marker = VersionMarker {
        app_version: app_version.to_string(),
        updated_at: now.to_string(),
    };
    let bytes = serde_json::to_vec_pretty(&marker)?;
    std::fs::write(version_marker(data_dir), bytes)
}

/// Decides whether this launch is the first one after an update.
///
/// Kept separate from the snapshot itself so the decision can be tested without
/// touching a filesystem. A folder with no marker but with real data in it is
/// treated as an upgrade from an unknown version — that is the case of someone
/// updating from a build that predates this module, which is exactly when a
/// snapshot is most valuable.
pub fn upgrade_reason(marker: Option<&VersionMarker>, current: &str, has_data: bool) -> Option<String> {
    match marker {
        Some(m) if m.app_version == current => None,
        Some(m) if m.app_version.is_empty() => {
            has_data.then(|| "update-from-unknown".to_string())
        }
        Some(m) => Some(format!("update-from-{}", m.app_version)),
        None => has_data.then(|| "update-from-unknown".to_string()),
    }
}

/// True when the folder holds anything worth keeping.
pub fn has_data(data_dir: &Path) -> bool {
    STATE_FILES.iter().any(|name| {
        std::fs::metadata(data_dir.join(name))
            .map(|m| m.len() > 2)
            .unwrap_or(false)
    })
}

/// Copies the state files into `backups/<id>/`. Returns the snapshot, or None
/// when there was nothing to copy.
pub fn take_snapshot(
    data_dir: &Path,
    id: &str,
    reason: &str,
    app_version: &str,
    now: &str,
) -> std::io::Result<Option<Snapshot>> {
    let target = backups_dir(data_dir).join(id);
    std::fs::create_dir_all(&target)?;

    let mut files = Vec::new();
    let mut bytes = 0u64;
    for name in STATE_FILES {
        let source = data_dir.join(name);
        if !source.is_file() {
            continue;
        }
        std::fs::copy(&source, target.join(name))?;
        bytes += std::fs::metadata(&source).map(|m| m.len()).unwrap_or(0);
        files.push((*name).to_string());
    }

    if files.is_empty() {
        let _ = std::fs::remove_dir(&target);
        return Ok(None);
    }

    let snapshot = Snapshot {
        id: id.to_string(),
        reason: reason.to_string(),
        app_version: app_version.to_string(),
        taken_at: now.to_string(),
        files,
        bytes,
    };
    std::fs::write(target.join("snapshot.json"), serde_json::to_vec_pretty(&snapshot)?)?;
    Ok(Some(snapshot))
}

/// Every snapshot on disk, newest first.
pub fn list_snapshots(data_dir: &Path) -> Vec<Snapshot> {
    let Ok(entries) = std::fs::read_dir(backups_dir(data_dir)) else {
        return Vec::new();
    };
    let mut found: Vec<Snapshot> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let text = std::fs::read_to_string(e.path().join("snapshot.json")).ok()?;
            serde_json::from_str::<Snapshot>(&text).ok()
        })
        .collect();
    // The id starts with a sortable timestamp, so this is newest-first.
    found.sort_by(|a, b| b.id.cmp(&a.id));
    found
}

/// Restores one snapshot over the live files, taking a snapshot of the current
/// state first — restoring the wrong one must itself be undoable.
pub fn restore_snapshot(
    data_dir: &Path,
    id: &str,
    safety_id: &str,
    app_version: &str,
    now: &str,
) -> std::io::Result<Vec<String>> {
    let source = backups_dir(data_dir).join(id);
    if !source.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no backup called {id}"),
        ));
    }

    take_snapshot(data_dir, safety_id, "before-restore", app_version, now)?;

    let mut restored = Vec::new();
    for name in STATE_FILES {
        let file = source.join(name);
        if !file.is_file() {
            continue;
        }
        std::fs::copy(&file, data_dir.join(name))?;
        restored.push((*name).to_string());
    }
    Ok(restored)
}

/// Keeps the newest `keep` snapshots and the update ones, deletes the rest.
///
/// Update snapshots are never pruned by count: they are the ones somebody will
/// want months later, when they notice something went missing.
pub fn prune(data_dir: &Path, keep: usize) -> Vec<String> {
    let all = list_snapshots(data_dir);
    let mut removed = Vec::new();
    let mut kept = 0usize;

    for snapshot in all {
        let permanent = snapshot.reason.starts_with("update-from");
        if permanent {
            continue;
        }
        kept += 1;
        if kept > keep {
            if std::fs::remove_dir_all(backups_dir(data_dir).join(&snapshot.id)).is_ok() {
                removed.push(snapshot.id);
            }
        }
    }
    removed
}

/// Moves a file that will not parse out of the way instead of overwriting it.
/// Returns where it went, so the user can be told the data still exists.
pub fn quarantine(path: &Path, now_slug: &str) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    let name = path.file_name()?.to_string_lossy().to_string();
    let target = path.with_file_name(format!("{name}.unreadable-{now_slug}"));
    std::fs::rename(path, &target).ok()?;
    Some(target)
}

/// Finds the newest snapshot that contains a readable copy of `name`.
pub fn newest_readable(data_dir: &Path, name: &str) -> Option<PathBuf> {
    list_snapshots(data_dir).into_iter().find_map(|snapshot| {
        let candidate = backups_dir(data_dir).join(&snapshot.id).join(name);
        let text = std::fs::read_to_string(&candidate).ok()?;
        serde_json::from_str::<serde_json::Value>(&text).ok()?;
        Some(candidate)
    })
}

/// A timestamp that sorts correctly and is legal in a Windows filename.
pub fn slug(now_iso: &str) -> String {
    now_iso.replace([':', '.'], "-").replace('T', "T")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gamehub-backup-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn the_same_version_is_not_an_upgrade() {
        let marker = VersionMarker { app_version: "0.3.0".into(), updated_at: String::new() };
        assert_eq!(upgrade_reason(Some(&marker), "0.3.0", true), None);
    }

    #[test]
    fn a_different_version_names_the_one_it_came_from() {
        let marker = VersionMarker { app_version: "0.2.0".into(), updated_at: String::new() };
        assert_eq!(
            upgrade_reason(Some(&marker), "0.3.0", true).as_deref(),
            Some("update-from-0.2.0")
        );
    }

    #[test]
    fn no_marker_but_real_data_is_an_upgrade_from_before_this_feature_existed() {
        assert_eq!(
            upgrade_reason(None, "0.3.0", true).as_deref(),
            Some("update-from-unknown")
        );
    }

    #[test]
    fn a_brand_new_install_is_not_an_upgrade() {
        // Nothing to protect, so no snapshot and no scary entry in the list.
        assert_eq!(upgrade_reason(None, "0.3.0", false), None);
    }

    #[test]
    fn an_empty_folder_has_no_data() {
        let dir = temp();
        assert!(!has_data(&dir));
    }

    #[test]
    fn an_empty_json_array_does_not_count_as_data() {
        // "[]" is what a fresh install writes. Snapshotting it would bury the
        // real backups under noise.
        let dir = temp();
        write(&dir, "library.json", "[]");
        assert!(!has_data(&dir));
    }

    #[test]
    fn a_real_library_counts_as_data() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"steam:400"}]"#);
        assert!(has_data(&dir));
    }

    #[test]
    fn a_snapshot_copies_every_state_file_that_exists() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"steam:400"}]"#);
        write(&dir, "settings.json", r#"{"theme":"dark"}"#);

        let snapshot = take_snapshot(&dir, "2026-01-01T00-00-00-test", "test", "0.3.0", "now")
            .unwrap()
            .expect("a snapshot");

        assert_eq!(snapshot.files.len(), 2);
        assert!(backups_dir(&dir).join(&snapshot.id).join("library.json").is_file());
        assert!(backups_dir(&dir).join(&snapshot.id).join("settings.json").is_file());
    }

    #[test]
    fn a_snapshot_of_nothing_is_not_created() {
        let dir = temp();
        assert!(take_snapshot(&dir, "id", "test", "0.3.0", "now").unwrap().is_none());
        assert!(!backups_dir(&dir).join("id").exists());
    }

    #[test]
    fn snapshots_are_listed_newest_first() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"a"}]"#);
        take_snapshot(&dir, "2026-01-01T00-00-00-a", "one", "0.1.0", "t1").unwrap();
        take_snapshot(&dir, "2026-06-01T00-00-00-b", "two", "0.2.0", "t2").unwrap();

        let listed = list_snapshots(&dir);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "2026-06-01T00-00-00-b");
    }

    #[test]
    fn restoring_brings_the_old_library_back() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"the-good-one"}]"#);
        take_snapshot(&dir, "2026-01-01T00-00-00-good", "update-from-0.1.0", "0.1.0", "t").unwrap();

        // Something goes wrong later and the library is replaced.
        write(&dir, "library.json", "[]");

        let restored = restore_snapshot(&dir, "2026-01-01T00-00-00-good", "safety", "0.3.0", "now").unwrap();
        assert!(restored.contains(&"library.json".to_string()));

        let back = std::fs::read_to_string(dir.join("library.json")).unwrap();
        assert!(back.contains("the-good-one"), "got {back}");
    }

    #[test]
    fn restoring_snapshots_the_current_state_first_so_it_can_be_undone() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"old"}]"#);
        take_snapshot(&dir, "2026-01-01T00-00-00-old", "manual", "0.1.0", "t").unwrap();
        write(&dir, "library.json", r#"[{"id":"current"}]"#);

        restore_snapshot(&dir, "2026-01-01T00-00-00-old", "2026-09-09T00-00-00-safety", "0.3.0", "now").unwrap();

        let safety = std::fs::read_to_string(
            backups_dir(&dir).join("2026-09-09T00-00-00-safety").join("library.json"),
        )
        .unwrap();
        assert!(safety.contains("current"), "the safety copy should hold what was live");
    }

    #[test]
    fn restoring_something_that_does_not_exist_is_an_error_not_a_wipe() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"keep-me"}]"#);
        assert!(restore_snapshot(&dir, "nope", "safety", "0.3.0", "now").is_err());
        let still = std::fs::read_to_string(dir.join("library.json")).unwrap();
        assert!(still.contains("keep-me"));
    }

    #[test]
    fn pruning_keeps_the_newest_and_never_deletes_an_update_snapshot() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"x"}]"#);
        take_snapshot(&dir, "2026-01-01T00-00-00-u", "update-from-0.1.0", "0.1.0", "t").unwrap();
        for day in 2..8 {
            take_snapshot(&dir, &format!("2026-01-0{day}T00-00-00-m"), "manual", "0.2.0", "t").unwrap();
        }

        let removed = prune(&dir, 3);
        let left = list_snapshots(&dir);

        assert!(!removed.is_empty(), "something should have been pruned");
        assert!(
            left.iter().any(|s| s.reason == "update-from-0.1.0"),
            "the update snapshot must survive"
        );
        assert_eq!(left.iter().filter(|s| s.reason == "manual").count(), 3);
    }

    #[test]
    fn a_corrupt_file_is_moved_aside_not_destroyed() {
        let dir = temp();
        write(&dir, "library.json", "{ this is not json");

        let moved = quarantine(&dir.join("library.json"), "2026-01-01T00-00-00").expect("moved");

        assert!(moved.is_file(), "the bad file must still exist");
        assert!(!dir.join("library.json").exists(), "the original name is now free");
        let body = std::fs::read_to_string(&moved).unwrap();
        assert_eq!(body, "{ this is not json", "byte-for-byte, so it can be repaired");
    }

    #[test]
    fn recovery_finds_the_newest_snapshot_that_actually_parses() {
        let dir = temp();
        write(&dir, "library.json", r#"[{"id":"good"}]"#);
        take_snapshot(&dir, "2026-01-01T00-00-00-a", "manual", "0.1.0", "t").unwrap();

        // A newer snapshot that is itself broken must be skipped, not chosen.
        let broken = backups_dir(&dir).join("2026-05-01T00-00-00-b");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("library.json"), "{ broken").unwrap();
        std::fs::write(
            broken.join("snapshot.json"),
            serde_json::to_vec(&Snapshot {
                id: "2026-05-01T00-00-00-b".into(),
                reason: "manual".into(),
                app_version: "0.2.0".into(),
                taken_at: "t".into(),
                files: vec!["library.json".into()],
                bytes: 8,
            })
            .unwrap(),
        )
        .unwrap();

        let found = newest_readable(&dir, "library.json").expect("a readable copy");
        let body = std::fs::read_to_string(found).unwrap();
        assert!(body.contains("good"));
    }

    #[test]
    fn recovery_returns_nothing_when_no_snapshot_has_the_file() {
        let dir = temp();
        assert!(newest_readable(&dir, "library.json").is_none());
    }

    #[test]
    fn backups_live_beside_the_data_folder_not_inside_it() {
        // An uninstaller that removes the data folder must not take the
        // backups with it.
        let data = PathBuf::from("/roaming/com.gamehub.desktop");
        let backups = backups_dir(&data);
        assert!(
            !backups.starts_with(&data),
            "{} must not be inside {}",
            backups.display(),
            data.display()
        );
        assert_eq!(backups, PathBuf::from("/roaming/com.gamehub.desktop Backups"));
    }

    #[test]
    fn a_snapshot_survives_the_data_folder_being_deleted() {
        // The scenario this whole module exists for, played out end to end.
        let base = temp();
        let data = base.join("com.gamehub.desktop");
        std::fs::create_dir_all(&data).unwrap();
        write(&data, "library.json", r#"[{"id":"my-games"}]"#);

        take_snapshot(&data, "2026-01-01T00-00-00-update-from-0.2.0", "update-from-0.2.0", "0.3.0", "t")
            .unwrap()
            .expect("a snapshot");

        // The installer wipes the app data folder during the update.
        std::fs::remove_dir_all(&data).unwrap();
        std::fs::create_dir_all(&data).unwrap();

        // The new version starts, finds nothing — and the backup is still there.
        assert!(!has_data(&data));
        let found = list_snapshots(&data);
        assert_eq!(found.len(), 1, "the snapshot must have survived");

        restore_snapshot(&data, &found[0].id, "safety", "0.3.0", "now").unwrap();
        let back = std::fs::read_to_string(data.join("library.json")).unwrap();
        assert!(back.contains("my-games"), "the library came back: {back}");
    }

    #[test]
    fn the_slug_is_a_legal_windows_filename() {
        let s = slug("2026-08-30T22:14:05.123Z");
        assert!(!s.contains(':'), "colons are illegal in Windows filenames: {s}");
        assert!(!s.contains('.'), "dots would confuse the extension: {s}");
    }
}
