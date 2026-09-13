//! Freeze points: the record of a game paused mid-scene, and the save copy
//! taken with it.
//!
//! What a freeze *is*: every thread of the game's processes suspended by the
//! OS, so the cutscene, the dialogue, the timer — whatever would not wait —
//! stops where it stands and continues from exactly there when resumed. That
//! part lives in the desktop app because it needs Windows. What is kept here is
//! everything around it: the index of freeze points, the folder each one
//! lives in, and the rules for what a freeze point can and cannot promise.
//!
//! Honesty about the limit: a suspended process lives only as long as the PC
//! stays on. A restart, or the game being closed, ends it. The save-folder copy
//! taken at the same moment is the part that survives, and the UI says so.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::saves::CopyReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FreezeState {
    /// The processes are suspended right now.
    Frozen,
    /// Resumed by the user; the save copy remains as a restore point.
    Resumed,
    /// The game is no longer running (closed, crashed, or the PC restarted).
    Gone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezePoint {
    pub id: String,
    pub game_id: Option<String>,
    pub game_name: String,
    pub frozen_at: String,
    pub state: FreezeState,
    /// The processes that were suspended. Empty for a save-only point.
    pub pids: Vec<u32>,
    /// The folder this point lives in — its save copy and picture are inside.
    pub folder: String,
    /// The game's save folder at the time, when one was known.
    pub save_dir: Option<String>,
    /// Where the save copy went, inside `folder`, when one was taken.
    pub save_copy: Option<String>,
    pub save_files: u64,
    pub save_bytes: u64,
    /// Files skipped for size, so nobody is surprised at restore time.
    #[serde(default)]
    pub skipped: Vec<String>,
    /// A screenshot of the moment, when the screen could be read.
    pub picture: Option<String>,
    #[serde(default)]
    pub note: String,
}

/// Everything about freezing that is remembered between runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FreezeLibrary {
    pub points: Vec<FreezePoint>,
    /// Game id → the save folder the user confirmed for it.
    pub save_dirs: std::collections::BTreeMap<String, String>,
}

/// The folder a new freeze point lives in: beside the replay clips, under the
/// game's own folder, so Explorer shows clips and freeze points together.
pub fn folder_for(clip_root: &Path, game_name: &str, frozen_at: &str) -> PathBuf {
    let game_folder = crate::media::folder_for(clip_root, game_name);
    game_folder.join(format!("Frys_{}", crate::backup::slug(frozen_at)))
}

/// Applies a copy's outcome to a point.
pub fn record_copy(point: &mut FreezePoint, copy_path: &Path, report: &CopyReport) {
    point.save_copy = Some(copy_path.to_string_lossy().into_owned());
    point.save_files = report.files;
    point.save_bytes = report.bytes;
    point.skipped = report.skipped.clone();
}

/// The freeze point currently holding `game_id` suspended, if any.
pub fn active_for<'a>(library: &'a FreezeLibrary, game_id: &str) -> Option<&'a FreezePoint> {
    library
        .points
        .iter()
        .find(|p| p.state == FreezeState::Frozen && p.game_id.as_deref() == Some(game_id))
}

/// Every point still marked frozen whose processes have all gone is marked
/// gone. `alive` answers whether a pid still exists. Returns how many changed.
pub fn reconcile(library: &mut FreezeLibrary, alive: impl Fn(u32) -> bool) -> usize {
    let mut changed = 0;
    for point in library.points.iter_mut().filter(|p| p.state == FreezeState::Frozen) {
        if point.pids.is_empty() || !point.pids.iter().any(|pid| alive(*pid)) {
            point.state = FreezeState::Gone;
            changed += 1;
        }
    }
    changed
}

/// Removes a point from the index, and its folder from disk when it lies
/// inside `clip_root` — never anywhere else.
pub fn remove(library: &mut FreezeLibrary, clip_root: &Path, id: &str) -> Result<(), String> {
    let Some(position) = library.points.iter().position(|p| p.id == id) else {
        return Err("@freeze_gone".into());
    };
    let folder = PathBuf::from(&library.points[position].folder);
    if folder.is_dir() {
        if !crate::safepath::is_within(clip_root, &folder) {
            return Err("@freeze_outside".into());
        }
        let _ = std::fs::remove_dir_all(&folder);
    }
    library.points.remove(position);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(id: &str, game: &str, state: FreezeState, pids: Vec<u32>) -> FreezePoint {
        FreezePoint {
            id: id.into(),
            game_id: Some(game.into()),
            game_name: game.into(),
            frozen_at: "2026-09-09T10:00:00Z".into(),
            state,
            pids,
            folder: String::new(),
            save_dir: None,
            save_copy: None,
            save_files: 0,
            save_bytes: 0,
            skipped: Vec::new(),
            picture: None,
            note: String::new(),
        }
    }

    #[test]
    fn freeze_points_live_beside_the_clips_under_the_game() {
        let folder = folder_for(Path::new("/clips"), "Elden Ring: Shadow", "2026-09-09T10:00:00Z");
        let text = folder.to_string_lossy().into_owned();
        assert!(text.starts_with("/clips/Elden Ring- Shadow"), "{text}");
        assert!(text.contains("Frys_"));
        assert!(!text.contains(':'), "a colon is not legal in a Windows folder name");
    }

    #[test]
    fn only_a_frozen_point_counts_as_active() {
        let mut library = FreezeLibrary::default();
        library.points.push(point("a", "steam:1", FreezeState::Resumed, vec![1]));
        library.points.push(point("b", "steam:1", FreezeState::Frozen, vec![2]));
        library.points.push(point("c", "steam:2", FreezeState::Frozen, vec![3]));
        assert_eq!(active_for(&library, "steam:1").map(|p| p.id.as_str()), Some("b"));
        assert!(active_for(&library, "steam:9").is_none());
    }

    #[test]
    fn a_frozen_point_whose_processes_vanished_is_marked_gone() {
        let mut library = FreezeLibrary::default();
        library.points.push(point("a", "g", FreezeState::Frozen, vec![10, 11]));
        library.points.push(point("b", "g", FreezeState::Frozen, vec![20]));
        library.points.push(point("c", "g", FreezeState::Resumed, vec![30]));
        let changed = reconcile(&mut library, |pid| pid == 11);
        assert_eq!(changed, 1);
        assert_eq!(library.points[0].state, FreezeState::Frozen, "one thread of it is still there");
        assert_eq!(library.points[1].state, FreezeState::Gone);
        assert_eq!(library.points[2].state, FreezeState::Resumed, "resumed points are left alone");
    }

    #[test]
    fn removing_deletes_the_folder_only_inside_the_clip_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Clips");
        let inside = root.join("Game").join("Frys_x");
        std::fs::create_dir_all(&inside).unwrap();
        let outside = tmp.path().join("elsewhere");
        std::fs::create_dir_all(&outside).unwrap();

        let mut library = FreezeLibrary::default();
        let mut a = point("a", "g", FreezeState::Gone, vec![]);
        a.folder = inside.to_string_lossy().into_owned();
        let mut b = point("b", "g", FreezeState::Gone, vec![]);
        b.folder = outside.to_string_lossy().into_owned();
        library.points.push(a);
        library.points.push(b);

        remove(&mut library, &root, "a").unwrap();
        assert!(!inside.exists());
        assert!(remove(&mut library, &root, "b").is_err());
        assert!(outside.exists(), "nothing outside the clip folder is ever deleted");
        assert!(remove(&mut library, &root, "zzz").is_err());
    }

    #[test]
    fn the_index_round_trips_through_json_with_old_fields_missing() {
        let text = r#"{"points":[{"id":"a","gameId":null,"gameName":"Desktop","frozenAt":"2026-01-01T00:00:00Z","state":"frozen","pids":[1],"folder":"/f","saveDir":null,"saveCopy":null,"saveFiles":0,"saveBytes":0,"picture":null}]}"#;
        let library: FreezeLibrary = serde_json::from_str(text).unwrap();
        assert_eq!(library.points.len(), 1);
        assert!(library.save_dirs.is_empty());
        let back = serde_json::to_string(&library).unwrap();
        assert!(back.contains("\"state\":\"frozen\""));
    }
}
