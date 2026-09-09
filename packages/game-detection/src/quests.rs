//! Quests and XP.
//!
//! Quests are generated from the library and the play history that already
//! exist — the games owned, what has been played lately, what has been ignored
//! for a month — so they are about the player's actual games rather than a
//! generic checklist.
//!
//! Progress is *derived*, never stored: a quest's state is recomputed from the
//! sessions inside its period every time it is read. That is what makes it
//! impossible to award yourself XP, and it means a quest completed while
//! GameHub was closed is already complete when it opens.
//!
//! Generation is deterministic: the same period always produces the same
//! quests, so restarting the app does not reroll them.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use time::Date;

use crate::activity::Session;
use crate::model::Game;

/// XP per level. Flat, so the number on screen is easy to reason about.
pub const XP_PER_LEVEL: u64 = 10_000;

pub const DAILY_XP: u64 = 200;
pub const BIWEEKLY_XP: u64 = 500;
pub const MONTHLY_XP: u64 = 1_000;

pub const DAILY_COUNT: usize = 1;
pub const BIWEEKLY_COUNT: usize = 2;
pub const MONTHLY_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestPeriod {
    Daily,
    Biweekly,
    Monthly,
}

impl QuestPeriod {
    pub fn xp(self) -> u64 {
        match self {
            QuestPeriod::Daily => DAILY_XP,
            QuestPeriod::Biweekly => BIWEEKLY_XP,
            QuestPeriod::Monthly => MONTHLY_XP,
        }
    }
}

/// What a quest actually asks for. Each one is checkable against the session
/// log with no extra bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Goal {
    /// Play one particular game for a while.
    #[serde(rename_all = "camelCase")]
    PlayGame { game_id: String, game_name: String, seconds: u64 },
    /// Play several different games.
    #[serde(rename_all = "camelCase")]
    DistinctGames { count: u64 },
    /// Any play at all, on this many separate days.
    #[serde(rename_all = "camelCase")]
    PlayOnDays { days: u64 },
    /// Total time across everything.
    #[serde(rename_all = "camelCase")]
    TotalPlaytime { seconds: u64 },
    /// Go back to something untouched for a while.
    #[serde(rename_all = "camelCase")]
    Revisit { game_id: String, game_name: String, seconds: u64 },
    /// Start something never played.
    #[serde(rename_all = "camelCase")]
    TrySomethingNew { seconds: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quest {
    /// Stable for the period, so progress and completion follow the same quest.
    pub id: String,
    pub period: QuestPeriod,
    pub title: String,
    pub description: String,
    pub goal: Goal,
    pub xp: u64,
    /// Progress towards `target`, in the goal's own unit.
    pub progress: u64,
    pub target: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestBoard {
    pub daily: Vec<Quest>,
    pub biweekly: Vec<Quest>,
    pub monthly: Vec<Quest>,
    pub xp: u64,
    pub level: u64,
    pub xp_into_level: u64,
    pub xp_for_level: u64,
}

/* -------------------------------------------------------------------------- */
/* Periods                                                                    */
/* -------------------------------------------------------------------------- */

/// The window a period covers, as inclusive ISO dates.
fn window(period: QuestPeriod, today: Date) -> (String, String, String) {
    match period {
        QuestPeriod::Daily => {
            let day = iso(today);
            (day.clone(), day.clone(), day)
        }
        QuestPeriod::Biweekly => {
            // Fortnights are counted from the start of the year so the boundary
            // never moves, whatever day the app is opened.
            let index = (today.ordinal() as u32 - 1) / 14;
            let start = Date::from_ordinal_date(today.year(), (index * 14 + 1) as u16).unwrap_or(today);
            let end = Date::from_ordinal_date(today.year(), ((index + 1) * 14).min(365) as u16).unwrap_or(today);
            (format!("{}-f{index}", today.year()), iso(start), iso(end))
        }
        QuestPeriod::Monthly => {
            let key = format!("{:04}-{:02}", today.year(), today.month() as u8);
            (key.clone(), format!("{key}-01"), format!("{key}-31"))
        }
    }
}

fn iso(date: Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month() as u8, date.day())
}

/// A small deterministic hash, so the same period always picks the same quests.
fn seed(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/* -------------------------------------------------------------------------- */
/* Generation                                                                 */
/* -------------------------------------------------------------------------- */

/// Builds the whole board: which quests, and how far along each one is.
pub fn board(games: &[Game], sessions: &[Session], today: Date) -> QuestBoard {
    let mut all: Vec<Quest> = Vec::new();
    for (period, count) in [
        (QuestPeriod::Daily, DAILY_COUNT),
        (QuestPeriod::Biweekly, BIWEEKLY_COUNT),
        (QuestPeriod::Monthly, MONTHLY_COUNT),
    ] {
        let (key, from, to) = window(period, today);
        let candidates = candidates(period, games, sessions, today);
        let chosen = pick(&candidates, count, &key);
        for (index, goal) in chosen.into_iter().enumerate() {
            let (progress, target) = evaluate(&goal, sessions, &from, &to);
            let (title, description) = describe(&goal, period);
            all.push(Quest {
                id: format!("{key}-{index}"),
                period,
                title,
                description,
                xp: period.xp(),
                complete: progress >= target,
                progress: progress.min(target),
                target,
                goal,
            });
        }
    }

    // XP is the sum of what is finished. Nothing else can add to it.
    let xp: u64 = all.iter().filter(|q| q.complete).map(|q| q.xp).sum();
    let level = xp / XP_PER_LEVEL + 1;

    QuestBoard {
        daily: all.iter().filter(|q| q.period == QuestPeriod::Daily).cloned().collect(),
        biweekly: all.iter().filter(|q| q.period == QuestPeriod::Biweekly).cloned().collect(),
        monthly: all.iter().filter(|q| q.period == QuestPeriod::Monthly).cloned().collect(),
        xp,
        level,
        xp_into_level: xp % XP_PER_LEVEL,
        xp_for_level: XP_PER_LEVEL,
    }
}

/// Every goal that makes sense for this player right now.
fn candidates(period: QuestPeriod, games: &[Game], sessions: &[Session], today: Date) -> Vec<Goal> {
    let installed: Vec<&Game> = games.iter().filter(|g| g.installed && !g.hidden).collect();
    if installed.is_empty() {
        return Vec::new();
    }

    let played: BTreeSet<&str> = sessions.iter().map(|s| s.game_id.as_str()).collect();
    let mut last_played: BTreeMap<&str, &str> = BTreeMap::new();
    for session in sessions {
        let entry = last_played.entry(session.game_id.as_str()).or_insert(&session.ended_at);
        if session.ended_at.as_str() > *entry {
            *entry = &session.ended_at;
        }
    }

    // A month ago, for "you have not touched this in a while".
    let cutoff = iso(today - time::Duration::days(30));

    let mut out = Vec::new();
    let minutes = match period {
        QuestPeriod::Daily => 30,
        QuestPeriod::Biweekly => 120,
        QuestPeriod::Monthly => 600,
    };

    // Favourites and recently played first — a quest about a game you like is
    // an invitation, not a chore — then the rest of the library, so a fresh
    // install with nothing played yet still gets a full board.
    let preferred = installed.iter().filter(|g| g.favorite || played.contains(g.id.as_str()));
    let rest = installed.iter().filter(|g| !(g.favorite || played.contains(g.id.as_str())));
    for game in preferred.chain(rest).take(16) {
        out.push(Goal::PlayGame {
            game_id: game.id.clone(),
            game_name: game.name.clone(),
            seconds: minutes * 60,
        });
    }

    for game in installed.iter().take(20) {
        match last_played.get(game.id.as_str()) {
            Some(last) if *last < cutoff.as_str() => out.push(Goal::Revisit {
                game_id: game.id.clone(),
                game_name: game.name.clone(),
                seconds: 30 * 60,
            }),
            _ => {}
        }
    }

    if installed.iter().any(|g| !played.contains(g.id.as_str())) {
        out.push(Goal::TrySomethingNew { seconds: 30 * 60 });
    }

    let library_size = installed.len() as u64;
    match period {
        QuestPeriod::Daily => {
            out.push(Goal::TotalPlaytime { seconds: 45 * 60 });
        }
        QuestPeriod::Biweekly => {
            out.push(Goal::DistinctGames { count: 2.min(library_size).max(1) });
            out.push(Goal::PlayOnDays { days: 4 });
            out.push(Goal::TotalPlaytime { seconds: 5 * 3600 });
        }
        QuestPeriod::Monthly => {
            out.push(Goal::DistinctGames { count: 3.min(library_size).max(1) });
            out.push(Goal::PlayOnDays { days: 12 });
            out.push(Goal::TotalPlaytime { seconds: 20 * 3600 });
        }
    }

    out
}

/// Picks `count` distinct goals, deterministically for the period, avoiding two
/// quests about the same game.
fn pick(candidates: &[Goal], count: usize, key: &str) -> Vec<Goal> {
    if candidates.is_empty() {
        return Vec::new();
    }
    let mut chosen: Vec<Goal> = Vec::new();
    let mut used_games: BTreeSet<String> = BTreeSet::new();
    let mut cursor = seed(key) as usize;

    for _ in 0..candidates.len() * 4 {
        if chosen.len() == count {
            break;
        }
        cursor = cursor.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let goal = &candidates[(cursor >> 33) % candidates.len()];
        if chosen.contains(goal) {
            continue;
        }
        if let Some(game) = goal_game(goal) {
            if !used_games.insert(game.to_string()) {
                continue;
            }
        }
        chosen.push(goal.clone());
    }

    // With a small library there may simply not be `count` distinct quests, and
    // padding with duplicates would be worse than a shorter board.
    chosen
}

fn goal_game(goal: &Goal) -> Option<&str> {
    match goal {
        Goal::PlayGame { game_id, .. } | Goal::Revisit { game_id, .. } => Some(game_id),
        _ => None,
    }
}

/* -------------------------------------------------------------------------- */
/* Progress                                                                   */
/* -------------------------------------------------------------------------- */

fn in_window<'a>(sessions: &'a [Session], from: &str, to: &str) -> Vec<&'a Session> {
    sessions
        .iter()
        .filter(|s| s.started_at.len() >= 10)
        .filter(|s| {
            let day = &s.started_at[..10];
            day >= from && day <= to
        })
        .collect()
}

/// Recomputes a goal's progress from the sessions in its period.
pub fn evaluate(goal: &Goal, sessions: &[Session], from: &str, to: &str) -> (u64, u64) {
    let window = in_window(sessions, from, to);

    match goal {
        Goal::PlayGame { game_id, seconds, .. } | Goal::Revisit { game_id, seconds, .. } => {
            let played = window.iter().filter(|s| &s.game_id == game_id).map(|s| s.seconds).sum();
            (played, *seconds)
        }
        Goal::DistinctGames { count } => {
            let distinct: BTreeSet<&str> = window.iter().map(|s| s.game_id.as_str()).collect();
            (distinct.len() as u64, *count)
        }
        Goal::PlayOnDays { days } => {
            let distinct: BTreeSet<&str> = window.iter().map(|s| &s.started_at[..10]).collect();
            (distinct.len() as u64, *days)
        }
        Goal::TotalPlaytime { seconds } => (window.iter().map(|s| s.seconds).sum(), *seconds),
        Goal::TrySomethingNew { seconds } => {
            // Something first played inside this window counts as new.
            let mut best = 0;
            let mut first_seen: BTreeMap<&str, &str> = BTreeMap::new();
            for session in sessions {
                let entry = first_seen.entry(session.game_id.as_str()).or_insert(&session.started_at);
                if session.started_at.as_str() < *entry {
                    *entry = &session.started_at;
                }
            }
            for (game_id, first) in &first_seen {
                if first.len() >= 10 && &first[..10] >= from && &first[..10] <= to {
                    let played: u64 = window
                        .iter()
                        .filter(|s| &s.game_id == game_id)
                        .map(|s| s.seconds)
                        .sum();
                    best = best.max(played);
                }
            }
            (best, *seconds)
        }
    }
}

fn describe(goal: &Goal, period: QuestPeriod) -> (String, String) {
    let span = match period {
        QuestPeriod::Daily => "today",
        QuestPeriod::Biweekly => "in the next two weeks",
        QuestPeriod::Monthly => "this month",
    };
    match goal {
        Goal::PlayGame { game_name, seconds, .. } => (
            game_name.clone(),
            format!("Play {game_name} for {} minutes {span}", seconds / 60),
        ),
        Goal::Revisit { game_name, seconds, .. } => (
            "Old friend".into(),
            format!(
                "You have not played {game_name} in a month. Give it {} minutes {span}",
                seconds / 60
            ),
        ),
        Goal::DistinctGames { count } => (
            "Variety".into(),
            format!("Play {count} different games {span}"),
        ),
        Goal::PlayOnDays { days } => (
            "Regular".into(),
            format!("Play on {days} separate days {span}"),
        ),
        Goal::TotalPlaytime { seconds } => (
            "Time well spent".into(),
            if *seconds >= 3600 {
                format!("Play for {} hours in total {span}", seconds / 3600)
            } else {
                format!("Play for {} minutes in total {span}", seconds / 60)
            },
        ),
        Goal::TrySomethingNew { seconds } => (
            "Something new".into(),
            format!(
                "Start a game you have never played and give it {} minutes {span}",
                seconds / 60
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GameSource;

    fn today() -> Date {
        Date::from_calendar_date(2026, time::Month::August, 30).unwrap()
    }

    fn game(id: &str, name: &str) -> Game {
        Game::new(GameSource::Steam, id, name)
    }

    fn session(date: &str, id: &str, name: &str, minutes: u64) -> Session {
        Session {
            game_id: format!("steam:{id}"),
            game_name: name.into(),
            started_at: format!("{date}T20:00:00Z"),
            ended_at: format!("{date}T21:00:00Z"),
            seconds: minutes * 60,
        }
    }

    #[test]
    fn the_board_has_the_counts_and_rewards_the_spec_asks_for() {
        let games: Vec<Game> = (1..=10).map(|i| game(&i.to_string(), &format!("Game {i}"))).collect();
        let board = board(&games, &[], today());

        assert_eq!(board.daily.len(), DAILY_COUNT);
        assert_eq!(board.biweekly.len(), BIWEEKLY_COUNT);
        assert_eq!(board.monthly.len(), MONTHLY_COUNT);
        assert!(board.daily.iter().all(|q| q.xp == 200));
        assert!(board.biweekly.iter().all(|q| q.xp == 500));
        assert!(board.monthly.iter().all(|q| q.xp == 1000));

        let possible: u64 = board.monthly.iter().map(|q| q.xp).sum();
        assert_eq!(possible, 5000, "five monthly quests at 1000 XP");
    }

    #[test]
    fn the_same_day_always_generates_the_same_quests() {
        let games: Vec<Game> = (1..=10).map(|i| game(&i.to_string(), &format!("Game {i}"))).collect();
        let first = board(&games, &[], today());
        let second = board(&games, &[], today());
        assert_eq!(first.daily[0].id, second.daily[0].id);
        assert_eq!(first.daily[0].goal, second.daily[0].goal, "restarting must not reroll the board");
    }

    #[test]
    fn a_different_day_generates_a_different_daily_quest() {
        let games: Vec<Game> = (1..=10).map(|i| game(&i.to_string(), &format!("Game {i}"))).collect();
        let a = board(&games, &[], today());
        let b = board(&games, &[], Date::from_calendar_date(2026, time::Month::August, 31).unwrap());
        assert_ne!(a.daily[0].id, b.daily[0].id);
    }

    #[test]
    fn a_fresh_install_with_nothing_played_still_gets_a_full_board() {
        let games: Vec<Game> = (1..=8).map(|i| game(&i.to_string(), &format!("Game {i}"))).collect();
        let board = board(&games, &[], today());
        assert_eq!(board.monthly.len(), MONTHLY_COUNT, "quests must not require a play history to exist");
    }

    #[test]
    fn two_quests_are_never_about_the_same_game() {
        let games = vec![game("1", "One"), game("2", "Two"), game("3", "Three")];
        let board = board(&games, &[], today());
        let mut named: Vec<&str> = board
            .monthly
            .iter()
            .filter_map(|q| goal_game(&q.goal))
            .collect();
        let before = named.len();
        named.sort();
        named.dedup();
        assert_eq!(named.len(), before, "a board must not ask twice about one game");
    }

    #[test]
    fn progress_is_computed_from_sessions_rather_than_stored() {
        let goal = Goal::PlayGame {
            game_id: "steam:1".into(),
            game_name: "One".into(),
            seconds: 30 * 60,
        };
        let sessions = vec![session("2026-08-30", "1", "One", 24)];
        let (progress, target) = evaluate(&goal, &sessions, "2026-08-30", "2026-08-30");
        assert_eq!(progress, 24 * 60);
        assert_eq!(target, 30 * 60);
    }

    #[test]
    fn a_quest_completes_on_its_own_with_no_claiming() {
        let games = vec![game("1", "One")];
        let sessions = vec![session("2026-08-30", "1", "One", 90)];
        let board = board(&games, &sessions, today());
        assert!(board.daily[0].complete, "playing enough is all it takes");
        assert!(board.xp >= DAILY_XP);
    }

    #[test]
    fn play_outside_the_period_does_not_count() {
        let goal = Goal::TotalPlaytime { seconds: 3600 };
        let sessions = vec![session("2026-08-01", "1", "One", 600)];
        let (progress, _) = evaluate(&goal, &sessions, "2026-08-30", "2026-08-30");
        assert_eq!(progress, 0);
    }

    #[test]
    fn distinct_games_counts_games_not_sessions() {
        let goal = Goal::DistinctGames { count: 3 };
        let sessions = vec![
            session("2026-08-30", "1", "One", 30),
            session("2026-08-30", "1", "One", 30),
            session("2026-08-30", "2", "Two", 30),
        ];
        let (progress, _) = evaluate(&goal, &sessions, "2026-08-01", "2026-08-31");
        assert_eq!(progress, 2);
    }

    #[test]
    fn revisit_only_appears_for_a_game_left_alone_for_a_month() {
        let mut games = vec![game("1", "Ancient"), game("2", "Current")];
        games[0].favorite = false;
        let sessions = vec![
            session("2026-06-01", "1", "Ancient", 60),
            session("2026-08-29", "2", "Current", 60),
        ];
        let goals = candidates(QuestPeriod::Monthly, &games, &sessions, today());
        let revisits: Vec<&Goal> = goals
            .iter()
            .filter(|g| matches!(g, Goal::Revisit { .. }))
            .collect();
        assert_eq!(revisits.len(), 1);
        assert!(matches!(revisits[0], Goal::Revisit { game_name, .. } if game_name == "Ancient"));
    }

    #[test]
    fn xp_only_comes_from_finished_quests() {
        let games = vec![game("1", "One")];
        let empty = board(&games, &[], today());
        assert_eq!(empty.xp, 0);
        assert_eq!(empty.level, 1);
    }

    #[test]
    fn levels_are_ten_thousand_xp_apart() {
        let games = vec![game("1", "One")];
        let mut board = board(&games, &[], today());
        board.xp = 17_400;
        board.level = board.xp / XP_PER_LEVEL + 1;
        board.xp_into_level = board.xp % XP_PER_LEVEL;
        assert_eq!(board.level, 2);
        assert_eq!(board.xp_into_level, 7_400);
    }

    #[test]
    fn an_empty_library_produces_an_empty_board_rather_than_impossible_quests() {
        let board = board(&[], &[], today());
        assert!(board.daily.is_empty() && board.monthly.is_empty());
        assert_eq!(board.level, 1);
    }
}
