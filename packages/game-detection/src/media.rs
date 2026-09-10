//! The screenshot library.
//!
//! A screenshot is a PNG on disk plus a row of metadata. The important part is
//! *which game it belongs to*: that comes from the same running-game watcher
//! everything else uses, never from a second guess at what is on screen. When
//! no game is running the shot is filed under "Desktop" rather than attributed
//! to whatever was played last.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Shots with no game attached. Named rather than left blank so the UI has
/// something honest to show.
pub const DESKTOP: &str = "Desktop";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub id: String,
    pub path: String,
    /// The game that was running, or `Desktop`.
    pub game_id: Option<String>,
    pub game_name: String,
    pub taken_at: String,
    pub size_bytes: u64,
    pub favorite: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenshotLibrary {
    pub items: Vec<Screenshot>,
}

/// Builds the file name a shot is saved under: sortable, unique, and safe on
/// Windows even when the game is called something awkward.
pub fn file_name(game_name: &str, taken_at: &str) -> String {
    let stamp: String = taken_at
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let game: String = game_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == ' ' { c } else { '-' })
        .collect::<String>()
        .trim()
        .replace(' ', "_");
    let game = if game.is_empty() { "Screenshot".to_string() } else { game };
    format!("{}_{}.png", game.chars().take(48).collect::<String>(), stamp)
}

/// Each game gets its own folder, so the library on disk reads the same way it
/// does in the app.
pub fn folder_for(root: &Path, game_name: &str) -> PathBuf {
    let safe: String = game_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == ' ' { c } else { '-' })
        .collect::<String>()
        .trim()
        .to_string();
    root.join(if safe.is_empty() { DESKTOP.to_string() } else { safe })
}

pub fn add(
    library: &mut ScreenshotLibrary,
    path: &Path,
    game: Option<(&str, &str)>,
    taken_at: &str,
    size_bytes: u64,
) -> Screenshot {
    let (game_id, game_name) = match game {
        Some((id, name)) => (Some(id.to_string()), name.to_string()),
        None => (None, DESKTOP.to_string()),
    };
    let shot = Screenshot {
        id: format!("{taken_at}-{}", library.items.len()),
        path: path.to_string_lossy().into_owned(),
        game_id,
        game_name,
        taken_at: taken_at.to_string(),
        size_bytes,
        favorite: false,
    };
    library.items.insert(0, shot.clone());
    shot
}

/// Drops entries whose file has been deleted outside GameHub, so the library
/// never shows a broken thumbnail.
pub fn prune_missing(library: &mut ScreenshotLibrary) -> usize {
    let before = library.items.len();
    library.items.retain(|s| Path::new(&s.path).is_file());
    before - library.items.len()
}

/// Grouped by game, newest group first, newest shot first inside each.
pub fn by_game(library: &ScreenshotLibrary) -> Vec<(String, Vec<Screenshot>)> {
    let mut groups: BTreeMap<String, Vec<Screenshot>> = BTreeMap::new();
    for shot in &library.items {
        groups.entry(shot.game_name.clone()).or_default().push(shot.clone());
    }
    let mut out: Vec<(String, Vec<Screenshot>)> = groups.into_iter().collect();
    for (_, shots) in out.iter_mut() {
        shots.sort_by(|a, b| b.taken_at.cmp(&a.taken_at));
    }
    // The game photographed most recently comes first.
    out.sort_by(|a, b| {
        let a_latest = a.1.first().map(|s| s.taken_at.clone()).unwrap_or_default();
        let b_latest = b.1.first().map(|s| s.taken_at.clone()).unwrap_or_default();
        b_latest.cmp(&a_latest)
    });
    out
}

pub fn set_favorite(library: &mut ScreenshotLibrary, id: &str, favorite: bool) -> bool {
    match library.items.iter_mut().find(|s| s.id == id) {
        Some(shot) => {
            shot.favorite = favorite;
            true
        }
        None => false,
    }
}

/// Removes the entry and the file. Only ever inside the screenshot root.
pub fn remove(library: &mut ScreenshotLibrary, root: &Path, id: &str) -> Result<(), String> {
    let Some(position) = library.items.iter().position(|s| s.id == id) else {
        return Err("@shot_gone".into());
    };
    let path = PathBuf::from(&library.items[position].path);
    if !crate::safepath::is_within(root, &path) {
        return Err("@shot_outside".into());
    }
    let _ = std::fs::remove_file(&path);
    library.items.remove(position);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_taken_while_a_game_runs_is_filed_under_that_game() {
        let mut library = ScreenshotLibrary::default();
        let shot = add(
            &mut library,
            Path::new("/shots/Celeste/a.png"),
            Some(("steam:504230", "Celeste")),
            "2026-08-30T16:42:00Z",
            120_000,
        );
        assert_eq!(shot.game_name, "Celeste");
        assert_eq!(shot.game_id.as_deref(), Some("steam:504230"));
    }

    #[test]
    fn a_shot_with_nothing_running_is_desktop_not_the_last_game() {
        let mut library = ScreenshotLibrary::default();
        add(&mut library, Path::new("/shots/a.png"), Some(("steam:1", "Celeste")), "2026-08-30T10:00:00Z", 1);
        let shot = add(&mut library, Path::new("/shots/b.png"), None, "2026-08-30T11:00:00Z", 1);
        assert_eq!(shot.game_name, DESKTOP);
        assert!(shot.game_id.is_none(), "never guess which game a desktop shot belongs to");
    }

    #[test]
    fn file_names_are_sortable_and_safe_on_windows() {
        let name = file_name("Grand Theft Auto V: Enhanced?", "2026-08-30T16:42:00Z");
        assert!(name.ends_with(".png"));
        for bad in [':', '?', '/', '\\', '*', '"', '<', '>', '|'] {
            assert!(!name.contains(bad), "{name} contains {bad}");
        }
        assert!(name.starts_with("Grand_Theft_Auto_V"));
    }

    #[test]
    fn each_game_gets_its_own_folder() {
        let root = Path::new("/shots");
        assert_eq!(folder_for(root, "Celeste"), root.join("Celeste"));
        assert_eq!(folder_for(root, ""), root.join(DESKTOP));
        assert!(!folder_for(root, "../etc").to_string_lossy().contains(".."));
    }

    #[test]
    fn grouping_puts_the_most_recently_photographed_game_first() {
        let mut library = ScreenshotLibrary::default();
        add(&mut library, Path::new("/a.png"), Some(("s:1", "Old")), "2026-08-01T10:00:00Z", 1);
        add(&mut library, Path::new("/b.png"), Some(("s:2", "New")), "2026-08-30T10:00:00Z", 1);
        let groups = by_game(&library);
        assert_eq!(groups[0].0, "New");
    }

    #[test]
    fn deleting_outside_the_screenshot_folder_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("shots");
        std::fs::create_dir_all(&root).unwrap();
        let outside = tmp.path().join("important.png");
        std::fs::write(&outside, b"x").unwrap();

        let mut library = ScreenshotLibrary::default();
        let shot = add(&mut library, &outside, None, "2026-08-30T10:00:00Z", 1);
        assert!(remove(&mut library, &root, &shot.id).is_err());
        assert!(outside.is_file(), "a file outside the folder is never deleted");
    }

    #[test]
    fn a_file_deleted_outside_gamehub_is_pruned_from_the_library() {
        let tmp = tempfile::tempdir().unwrap();
        let present = tmp.path().join("here.png");
        std::fs::write(&present, b"x").unwrap();

        let mut library = ScreenshotLibrary::default();
        add(&mut library, &present, None, "2026-08-30T10:00:00Z", 1);
        add(&mut library, &tmp.path().join("gone.png"), None, "2026-08-30T11:00:00Z", 1);

        assert_eq!(prune_missing(&mut library), 1);
        assert_eq!(library.items.len(), 1);
    }
}
