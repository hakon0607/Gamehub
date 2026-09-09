//! Merging a scan into the stored library.
//!
//! A scan produces what is installed *now*. The library holds what the user has
//! decided *about* those games — favourites, tags, hidden, custom names — plus
//! the metadata GameHub fetched. Merging has to keep the second while replacing
//! the first, and it has to be able to say which games are new, because that is
//! what drives the "new game detected" notification.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Game, GameMetadata};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    /// Games seen for the first time, in scan order.
    pub new_ids: Vec<String>,
    /// Games that were installed and no longer are.
    pub uninstalled_ids: Vec<String>,
}

/// User-owned fields. These survive every rescan; a scan may never write them.
fn carry_user_fields(previous: &Game, next: &mut Game) {
    next.favorite = previous.favorite;
    next.hidden = previous.hidden;
    next.tags = previous.tags.clone();
    next.discovered_at = previous.discovered_at.clone();

    // Metadata is expensive to fetch, so it is kept unless the scan brought
    // something better.
    if next.metadata.is_none() {
        next.metadata = previous.metadata.clone();
    }

    // A launcher that stops reporting playtime must not erase the number the
    // user could see yesterday.
    if next.playtime_seconds.is_none() {
        next.playtime_seconds = previous.playtime_seconds;
    }
    if next.last_played.is_none() {
        next.last_played = previous.last_played.clone();
    }

    // A game the user renamed keeps its name. Adapters set `name` from the
    // launcher every time, which would otherwise undo the rename on each scan.
    if previous.tags.iter().any(|t| t == "renamed") {
        next.name = previous.name.clone();
    }
}

/// Folds a scan's games into the stored library.
///
/// Games the scan did not see are kept but marked uninstalled, rather than
/// deleted: an external drive being unplugged should not throw away the
/// playtime, favourites and artwork for forty games.
pub fn merge(stored: &mut Vec<Game>, scanned: Vec<Game>) -> MergeOutcome {
    let mut outcome = MergeOutcome::default();
    let by_id: BTreeMap<String, usize> = stored
        .iter()
        .enumerate()
        .map(|(i, g)| (g.id.clone(), i))
        .collect();

    let mut seen: BTreeSet<String> = BTreeSet::new();

    for mut game in scanned {
        seen.insert(game.id.clone());
        match by_id.get(&game.id) {
            Some(&index) => {
                carry_user_fields(&stored[index], &mut game);
                stored[index] = game;
            }
            None => {
                outcome.new_ids.push(game.id.clone());
                stored.push(game);
            }
        }
    }

    for game in stored.iter_mut() {
        if game.installed && !seen.contains(&game.id) {
            game.installed = false;
            game.launch = None;
            outcome.uninstalled_ids.push(game.id.clone());
        }
    }

    stored.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    outcome
}

/// Applies fetched metadata without disturbing anything else about the game.
pub fn apply_metadata(library: &mut [Game], game_id: &str, metadata: GameMetadata) -> bool {
    match library.iter_mut().find(|g| g.id == game_id) {
        Some(game) => {
            game.metadata = Some(metadata);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GameSource;

    fn game(source: GameSource, id: &str, name: &str) -> Game {
        Game::new(source, id, name)
    }

    #[test]
    fn a_first_scan_reports_everything_as_new() {
        let mut library = Vec::new();
        let outcome = merge(
            &mut library,
            vec![game(GameSource::Steam, "440", "Team Fortress 2"), game(GameSource::Epic, "Fortnite", "Fortnite")],
        );
        assert_eq!(outcome.new_ids.len(), 2);
        assert_eq!(library.len(), 2);
    }

    #[test]
    fn a_second_scan_reports_only_the_game_that_appeared() {
        let mut library = vec![game(GameSource::Steam, "440", "Team Fortress 2")];
        let outcome = merge(
            &mut library,
            vec![
                game(GameSource::Steam, "440", "Team Fortress 2"),
                game(GameSource::Steam, "1174180", "Red Dead Redemption 2"),
            ],
        );
        assert_eq!(outcome.new_ids, vec!["steam:1174180".to_string()]);
    }

    #[test]
    fn favourites_tags_and_artwork_survive_a_rescan() {
        let mut first = game(GameSource::Steam, "440", "Team Fortress 2");
        first.favorite = true;
        first.tags = vec!["shooter".into()];
        first.metadata = Some(GameMetadata {
            cover_path: Some("covers/440.jpg".into()),
            ..Default::default()
        });
        first.discovered_at = "2020-01-01T00:00:00Z".into();
        let mut library = vec![first];

        merge(&mut library, vec![game(GameSource::Steam, "440", "Team Fortress 2")]);

        let stored = &library[0];
        assert!(stored.favorite);
        assert_eq!(stored.tags, vec!["shooter".to_string()]);
        assert_eq!(stored.metadata.as_ref().unwrap().cover_path.as_deref(), Some("covers/440.jpg"));
        assert_eq!(stored.discovered_at, "2020-01-01T00:00:00Z");
    }

    #[test]
    fn an_unplugged_drive_marks_games_uninstalled_rather_than_deleting_them() {
        let mut library = vec![game(GameSource::Steam, "440", "Team Fortress 2")];
        library[0].favorite = true;
        let outcome = merge(&mut library, vec![]);

        assert_eq!(outcome.uninstalled_ids, vec!["steam:440".to_string()]);
        assert_eq!(library.len(), 1);
        assert!(!library[0].installed);
        assert!(library[0].launch.is_none(), "an uninstalled game must not keep a launch path");
        assert!(library[0].favorite, "the user's favourite survives the drive being unplugged");
    }

    #[test]
    fn a_reinstalled_game_is_not_reported_as_new_again() {
        let mut library = vec![game(GameSource::Steam, "440", "Team Fortress 2")];
        merge(&mut library, vec![]);
        let outcome = merge(&mut library, vec![game(GameSource::Steam, "440", "Team Fortress 2")]);
        assert!(outcome.new_ids.is_empty());
        assert!(library[0].installed);
    }

    #[test]
    fn playtime_is_not_erased_by_a_launcher_that_stops_reporting_it() {
        let mut previous = game(GameSource::Steam, "440", "Team Fortress 2");
        previous.playtime_seconds = Some(3600);
        previous.last_played = Some("2026-01-01T00:00:00Z".into());
        let mut library = vec![previous];

        merge(&mut library, vec![game(GameSource::Steam, "440", "Team Fortress 2")]);
        assert_eq!(library[0].playtime_seconds, Some(3600));
        assert_eq!(library[0].last_played.as_deref(), Some("2026-01-01T00:00:00Z"));
    }

    #[test]
    fn the_same_game_in_two_launchers_stays_two_entries() {
        let mut library = Vec::new();
        merge(
            &mut library,
            vec![game(GameSource::Steam, "1", "Rocket League"), game(GameSource::Epic, "Sugar", "Rocket League")],
        );
        assert_eq!(library.len(), 2, "ids are per source, so a game owned twice is listed twice");
    }
}
