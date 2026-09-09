//! Where a game keeps its save files — and copying them aside.
//!
//! This is what makes "freeze the game now" worth more than a pause button. A
//! frozen process is gone the moment the PC restarts; a copy of the save folder
//! taken at the same instant is not. Games put their saves in half a dozen
//! places and never say which, so this module *looks*: the usual Windows
//! folders, matched by name against the game, plus a few conventional
//! subfolders of the install directory. The candidates are shown to the user
//! to confirm, because a guess that copies the wrong folder is worse than no
//! guess.
//!
//! Nothing here is Windows-specific; it runs against a fixture tree in tests.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::env::KnownFolders;

/// A folder that might be a game's save location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCandidate {
    pub path: String,
    /// Where it was found — "Saved Games", "AppData", "Documents", "install folder"…
    pub location: String,
    pub files: u64,
    pub bytes: u64,
    /// The newest file's modification time, as RFC 3339. A folder touched
    /// recently is far more likely to be the live save location.
    pub modified_at: Option<String>,
    /// 0–100. How sure the matcher is; the UI sorts by it.
    pub confidence: u8,
}

/// Files larger than this are never part of a snapshot: a save folder that
/// contains a 4 GB cache is not one anyone wants copied on every freeze.
pub const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
/// And the whole snapshot stops here.
pub const MAX_SNAPSHOT_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Lower-case letters and digits only, so "Marvel's Spider-Man: Remastered"
/// and "MarvelsSpiderManRemastered" compare equal.
pub fn normalise(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The publisher-ish prefixes and edition suffixes that keep a folder from
/// matching a title exactly.
fn strip_editions(normalised: &str) -> String {
    let mut out = normalised.to_string();
    for suffix in [
        "remastered",
        "enhancededition",
        "definitiveedition",
        "goldedition",
        "deluxeedition",
        "gameoftheyearedition",
        "goty",
        "edition",
    ] {
        if out.ends_with(suffix) && out.len() > suffix.len() + 2 {
            out.truncate(out.len() - suffix.len());
        }
    }
    out
}

/// How well a folder name matches a game name, 0–100.
///
/// Exact is 100; the folder name containing the whole game name (or the other
/// way round) scores by how much of the longer one is covered; a short name
/// or one covering under half is no match, since "Rust" would otherwise claim
/// "Rustler" and "the" half the disk.
pub fn name_score(game: &str, folder: &str) -> u8 {
    let g = strip_editions(&normalise(game));
    let f = strip_editions(&normalise(folder));
    if g.is_empty() || f.is_empty() {
        return 0;
    }
    if g == f {
        return 100;
    }
    let (short, long) = if g.len() <= f.len() { (&g, &f) } else { (&f, &g) };
    if short.len() < 5 {
        return 0;
    }
    if long.contains(short.as_str()) {
        let coverage = short.len() as f64 / long.len() as f64;
        if coverage < 0.5 {
            return 0;
        }
        return (55.0 + coverage * 40.0) as u8;
    }
    0
}

/// Subfolders of an install directory that are conventionally saves.
const INSTALL_SAVE_FOLDERS: &[&str] = &[
    "save", "saves", "savegame", "savegames", "SaveGames", "Saved", "profiles", "SaveData", "savedata",
];

fn folder_stats(dir: &Path) -> (u64, u64, Option<SystemTime>) {
    let mut files = 0;
    let mut bytes = 0;
    let mut newest: Option<SystemTime> = None;
    walk(dir, 0, &mut |path, meta| {
        files += 1;
        bytes += meta.len();
        if let Ok(modified) = meta.modified() {
            newest = Some(newest.map_or(modified, |n| n.max(modified)));
        }
        let _ = path;
    });
    (files, bytes, newest)
}

/// Depth-limited walk: save folders are shallow, and a runaway walk into a
/// 100 000-file mod directory would freeze the UI.
fn walk(dir: &Path, depth: usize, visit: &mut dyn FnMut(&Path, &std::fs::Metadata)) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk(&path, depth + 1, visit);
        } else if meta.is_file() {
            visit(&path, &meta);
        }
    }
}

fn iso(time: Option<SystemTime>) -> Option<String> {
    let seconds = time?.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs() as i64;
    crate::unix_to_iso8601(seconds)
}

/// Every folder that might hold `game_name`'s saves, best first.
pub fn find_candidates(folders: &KnownFolders, game_name: &str, install_dir: Option<&Path>) -> Vec<SaveCandidate> {
    let documents = folders.user_profile.join("Documents");
    let roots: Vec<(PathBuf, &str)> = vec![
        (folders.user_profile.join("Saved Games"), "Saved Games"),
        (documents.join("My Games"), "Documents\\My Games"),
        (documents.clone(), "Documents"),
        (folders.app_data.clone(), "AppData\\Roaming"),
        (folders.local_app_data.clone(), "AppData\\Local"),
        (folders.local_app_data.join("..").join("LocalLow"), "AppData\\LocalLow"),
        (folders.user_profile.join("OneDrive").join("Documents"), "OneDrive\\Documents"),
    ];

    let mut found: Vec<SaveCandidate> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut consider = |path: PathBuf, location: &str, confidence: u8| {
        // Lexically normalised rather than canonicalised: on Windows the
        // latter yields a `\\?\` path nobody wants to see in a list.
        let canonical = crate::safepath::normalise(&path);
        if seen.contains(&canonical) || !canonical.is_dir() {
            return;
        }
        seen.push(canonical.clone());
        let (files, bytes, newest) = folder_stats(&canonical);
        if files == 0 {
            return;
        }
        found.push(SaveCandidate {
            path: canonical.to_string_lossy().into_owned(),
            location: location.to_string(),
            files,
            bytes,
            modified_at: iso(newest),
            confidence,
        });
    };

    for (root, location) in &roots {
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let score = name_score(game_name, &name);
            if score == 0 {
                // One level deeper: "Documents\Rockstar Games\GTA V", or a
                // publisher folder in AppData.
                if let Ok(children) = std::fs::read_dir(&path) {
                    for child in children.flatten() {
                        let child_path = child.path();
                        if !child_path.is_dir() {
                            continue;
                        }
                        let child_name = child.file_name().to_string_lossy().into_owned();
                        let child_score = name_score(game_name, &child_name);
                        if child_score > 0 {
                            consider(child_path, location, child_score.saturating_sub(5));
                        }
                    }
                }
                continue;
            }
            consider(path, location, score);
        }
    }

    if let Some(install) = install_dir {
        for folder in INSTALL_SAVE_FOLDERS {
            let path = install.join(folder);
            if path.is_dir() {
                consider(path, "install folder", 60);
            }
        }
    }

    // Best first; ties broken by whichever was touched most recently.
    found.sort_by(|a, b| b.confidence.cmp(&a.confidence).then_with(|| b.modified_at.cmp(&a.modified_at)));
    found
}

/// What a copy did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyReport {
    pub files: u64,
    pub bytes: u64,
    /// Files left out for being over the size cap, named so the user knows.
    pub skipped: Vec<String>,
}

/// Copies `from` into `to`, recursively, honouring the size caps.
///
/// `to` is created. Nothing in `from` is touched. The relative layout is kept
/// exactly, so putting the copy back is a plain reverse copy.
pub fn copy_tree(from: &Path, to: &Path) -> std::io::Result<CopyReport> {
    let mut report = CopyReport::default();
    std::fs::create_dir_all(to)?;
    copy_into(from, to, &mut report, 0)?;
    Ok(report)
}

fn copy_into(from: &Path, to: &Path, report: &mut CopyReport, depth: usize) -> std::io::Result<()> {
    if depth > 6 {
        return Ok(());
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let meta = entry.metadata()?;
        if meta.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_into(&source, &target, report, depth + 1)?;
        } else if meta.is_file() {
            if meta.len() > MAX_FILE_BYTES || report.bytes + meta.len() > MAX_SNAPSHOT_BYTES {
                report.skipped.push(source.to_string_lossy().into_owned());
                continue;
            }
            std::fs::copy(&source, &target)?;
            report.files += 1;
            report.bytes += meta.len();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_compared_without_punctuation_case_or_edition() {
        assert_eq!(name_score("Marvel's Spider-Man: Remastered", "MarvelsSpiderMan"), 100);
        assert_eq!(name_score("Elden Ring", "EldenRing"), 100);
        assert_eq!(name_score("Cyberpunk 2077", "CD Projekt Red"), 0);
    }

    #[test]
    fn a_folder_that_contains_the_title_scores_less_than_exact_but_still_matches() {
        let partial = name_score("Hades", "Hades II");
        assert!(partial > 0 && partial < 100, "got {partial}");
        assert!(name_score("The Witcher 3", "The Witcher 3 Wild Hunt") > 70);
    }

    #[test]
    fn short_words_do_not_match_everything() {
        // "Rust" must not claim "Trust" or "Rustler".
        assert_eq!(name_score("Rust", "Rustler"), 0);
        assert_eq!(name_score("It", "Ittle Dew"), 0);
    }

    fn touch(path: &Path, bytes: usize) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![0u8; bytes]).unwrap();
    }

    #[test]
    fn candidates_are_found_in_the_usual_places_best_first() {
        let tmp = tempfile::tempdir().unwrap();
        let folders = KnownFolders::rooted_at(tmp.path());
        let home = &folders.user_profile;

        touch(&home.join("Saved Games/Elden Ring/save.sl2"), 100);
        touch(&home.join("Documents/My Games/Elden Ring Mods/x.txt"), 10);
        touch(&folders.app_data.join("EldenRing/GR/steam/data.bin"), 50);
        touch(&folders.local_app_data.join("Unrelated/thing.dat"), 10);
        let install = tmp.path().join("Games/ELDEN RING");
        touch(&install.join("saves/slot1.bin"), 20);

        let found = find_candidates(&folders, "ELDEN RING", Some(&install));
        let paths: Vec<&str> = found.iter().map(|c| c.location.as_str()).collect();
        assert!(paths.contains(&"Saved Games"), "{found:?}");
        assert!(paths.contains(&"AppData\\Roaming"), "{found:?}");
        assert!(paths.contains(&"install folder"), "{found:?}");
        assert!(!found.iter().any(|c| c.path.contains("Unrelated")));
        assert_eq!(found[0].confidence, 100);
        assert!(found[0].files > 0 && found[0].bytes > 0);
    }

    #[test]
    fn a_publisher_folder_one_level_down_is_searched_too() {
        let tmp = tempfile::tempdir().unwrap();
        let folders = KnownFolders::rooted_at(tmp.path());
        touch(&folders.user_profile.join("Documents/Rockstar Games/GTA V/Profiles/abc/SGTA50000"), 10);
        let found = find_candidates(&folders, "GTA V", None);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].path.ends_with("GTA V"));
        assert!(found[0].confidence >= 90);
    }

    #[test]
    fn empty_folders_are_not_offered() {
        let tmp = tempfile::tempdir().unwrap();
        let folders = KnownFolders::rooted_at(tmp.path());
        std::fs::create_dir_all(folders.user_profile.join("Saved Games/Hades")).unwrap();
        assert!(find_candidates(&folders, "Hades", None).is_empty());
    }

    #[test]
    fn nothing_on_the_machine_is_an_empty_list_not_an_error() {
        let folders = KnownFolders::rooted_at("/definitely/not/here");
        assert!(find_candidates(&folders, "Anything", None).is_empty());
    }

    #[test]
    fn a_copy_keeps_the_layout_and_reports_what_it_did() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from");
        touch(&from.join("a.sav"), 3);
        touch(&from.join("deep/b.sav"), 5);
        let to = tmp.path().join("to");

        let report = copy_tree(&from, &to).unwrap();
        assert_eq!(report.files, 2);
        assert_eq!(report.bytes, 8);
        assert!(report.skipped.is_empty());
        assert!(to.join("a.sav").is_file());
        assert!(to.join("deep/b.sav").is_file());
        // The source is untouched.
        assert!(from.join("a.sav").is_file());
    }

    #[test]
    fn copying_back_restores_exactly() {
        let tmp = tempfile::tempdir().unwrap();
        let live = tmp.path().join("live");
        touch(&live.join("slot.sav"), 4);
        let snapshot = tmp.path().join("snap");
        copy_tree(&live, &snapshot).unwrap();
        // The game then overwrites its save.
        std::fs::write(live.join("slot.sav"), b"changed!").unwrap();
        copy_tree(&snapshot, &live).unwrap();
        assert_eq!(std::fs::read(live.join("slot.sav")).unwrap(), vec![0u8; 4]);
    }
}
