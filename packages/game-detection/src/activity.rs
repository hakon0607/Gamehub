//! Play activity: sessions, days, streaks and the calendar.
//!
//! There is exactly one source of truth for "is a game running" — the process
//! watcher — and everything here is derived from it. A session opens when a
//! game's executable appears among the running processes and closes when it
//! disappears. Days, streaks, the calendar and the "recently played" list are
//! all views over the same session list rather than separate trackers.
//!
//! Playtime is *active* time, not wall time. The watcher says, on every tick,
//! whether each running game is actually being played — its window in front
//! and the player not idle — and only those ticks add up. A game left open in
//! the background, a frozen game, or a PC that went to sleep with the game
//! running adds nothing. The session still remembers when it started and
//! ended, so the calendar can show the whole evening.
//!
//! The whole module is pure: it takes the set of running game ids and a
//! timestamp and returns the new state. That is what makes it testable without
//! a running game, and it is why the tests below can simulate a week of play in
//! a millisecond.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime};

/// A finished period of play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub game_id: String,
    pub game_name: String,
    /// RFC 3339, UTC.
    pub started_at: String,
    pub ended_at: String,
    /// Seconds of active play — the game in front, the player at the keyboard.
    pub seconds: u64,
    /// Seconds the game was open in total, for the curious. 0 for sessions
    /// recorded before active tracking existed.
    #[serde(default)]
    pub wall_seconds: u64,
}

/// A session that has not finished yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSession {
    pub game_id: String,
    pub game_name: String,
    pub started_at: String,
    /// Active seconds counted so far.
    #[serde(default)]
    pub active_seconds: u64,
    /// When the watcher last looked, so the next tick knows how much time
    /// passed. Empty for a session opened by an older version.
    #[serde(default)]
    pub last_tick: String,
    /// Whether the last tick saw the game being played.
    #[serde(default)]
    pub active: bool,
}

/// One running game as the watcher sees it on a tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Running {
    pub game_id: String,
    pub game_name: String,
    /// The game is the window in front and the player is not idle.
    pub active: bool,
}

/// The longest gap between two ticks that still counts. The watcher looks
/// every ten seconds; a gap far longer than that means the PC was asleep, and
/// sleeping is not playing.
pub const MAX_TICK_SECONDS: u64 = 60;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Activity {
    pub sessions: Vec<Session>,
    pub open: Vec<OpenSession>,
    /// Minutes in a day before it counts towards a streak. The user can change it.
    pub streak_threshold_minutes: u64,
    /// When false, nothing new is recorded. Launching games still works.
    pub tracking_enabled: bool,
}

impl Activity {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            open: Vec::new(),
            streak_threshold_minutes: 15,
            tracking_enabled: true,
        }
    }
}

/// A session that just ended, so the caller can announce it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedSession(pub Session);

/// Folds the current set of running games into the activity log.
///
/// `now` is passed in rather than read from the clock so the tests can move
/// time deliberately. Sessions shorter than a minute are discarded: alt-tabbing
/// through a launcher should not litter the calendar.
pub fn observe(
    activity: &mut Activity,
    running: &[(String, String)],
    now: OffsetDateTime,
) -> Vec<ClosedSession> {
    let seen: Vec<Running> = running
        .iter()
        .map(|(id, name)| Running { game_id: id.clone(), game_name: name.clone(), active: true })
        .collect();
    observe_active(activity, &seen, now)
}

/// Like [`observe`], but told which of the running games are actually being
/// played right now. Only active ticks add to a session's playtime.
pub fn observe_active(activity: &mut Activity, running: &[Running], now: OffsetDateTime) -> Vec<ClosedSession> {
    if !activity.tracking_enabled {
        // Anything still open when tracking is switched off is closed cleanly
        // rather than left dangling forever.
        return close_all(activity, now);
    }

    let mut closed = Vec::new();

    // Games that stopped, and ticks for the ones still going.
    let mut remaining = Vec::new();
    for mut open in std::mem::take(&mut activity.open) {
        match running.iter().find(|r| r.game_id == open.game_id) {
            Some(seen) => {
                tick(&mut open, seen.active, now);
                remaining.push(open);
            }
            None => {
                if let Some(session) = finish(&open, now) {
                    activity.sessions.push(session.clone());
                    closed.push(ClosedSession(session));
                }
            }
        }
    }
    activity.open = remaining;

    // Games that started.
    for seen in running {
        if !activity.open.iter().any(|o| o.game_id == seen.game_id) {
            activity.open.push(OpenSession {
                game_id: seen.game_id.clone(),
                game_name: seen.game_name.clone(),
                started_at: format_time(now),
                active_seconds: 0,
                last_tick: format_time(now),
                active: seen.active,
            });
        }
    }

    closed
}

/// Adds the time since the last tick when the game was being played at both
/// ends of it. A gap longer than [`MAX_TICK_SECONDS`] is a sleep, not play.
fn tick(open: &mut OpenSession, active_now: bool, now: OffsetDateTime) {
    let elapsed = parse_time(&open.last_tick)
        .map(|last| (now - last).whole_seconds().max(0) as u64)
        .unwrap_or(0);
    if active_now && open.active && elapsed <= MAX_TICK_SECONDS {
        open.active_seconds += elapsed;
    }
    open.active = active_now;
    open.last_tick = format_time(now);
}

/// Active seconds of an open session as of `now`, for a live display.
pub fn active_so_far(open: &OpenSession, now: OffsetDateTime) -> u64 {
    let since_tick = parse_time(&open.last_tick)
        .map(|last| (now - last).whole_seconds().max(0) as u64)
        .unwrap_or(0);
    open.active_seconds + if open.active && since_tick <= MAX_TICK_SECONDS { since_tick } else { 0 }
}

/// Sessions no human played: longer than this and recorded by a version
/// that counted wall time (no `wall_seconds`), they are a game left open
/// overnight, not play.
pub const IMPOSSIBLE_SESSION_SECONDS: u64 = 12 * 3600;

/// Drops sessions recorded before active tracking existed that are too long
/// to be real. Returns how many went. Sessions from the new method are kept
/// whatever their length — they were measured, not assumed.
pub fn prune_impossible(activity: &mut Activity) -> usize {
    let before = activity.sessions.len();
    activity
        .sessions
        .retain(|s| !(s.wall_seconds == 0 && s.seconds > IMPOSSIBLE_SESSION_SECONDS));
    before - activity.sessions.len()
}

/// Ends every open session — used when tracking is disabled and at shutdown, so
/// a crash is the only way to lose a session rather than the normal path.
pub fn close_all(activity: &mut Activity, now: OffsetDateTime) -> Vec<ClosedSession> {
    let mut closed = Vec::new();
    for open in std::mem::take(&mut activity.open) {
        if let Some(session) = finish(&open, now) {
            activity.sessions.push(session.clone());
            closed.push(ClosedSession(session));
        }
    }
    closed
}

/// Sessions under a minute are noise, not play.
const MINIMUM_SESSION_SECONDS: u64 = 60;

fn finish(open: &OpenSession, now: OffsetDateTime) -> Option<Session> {
    let started = parse_time(&open.started_at)?;
    let wall_seconds = (now - started).whole_seconds().max(0) as u64;
    let seconds = active_so_far(open, now);
    if seconds < MINIMUM_SESSION_SECONDS {
        return None;
    }
    Some(Session {
        game_id: open.game_id.clone(),
        game_name: open.game_name.clone(),
        started_at: open.started_at.clone(),
        ended_at: format_time(now),
        seconds,
        wall_seconds,
    })
}

/* -------------------------------------------------------------------------- */
/* Views over the sessions                                                    */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    /// ISO date, e.g. "2026-08-30".
    pub date: String,
    pub seconds: u64,
    /// Game name to seconds, most played first.
    pub games: Vec<(String, u64)>,
    pub session_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreakSummary {
    pub current: u64,
    pub longest: u64,
    pub total_days: u64,
    pub total_seconds: u64,
    /// Games played during the current streak, most played first.
    pub current_streak_games: Vec<String>,
    /// "2026-08" and its seconds, for "best month".
    pub best_month: Option<(String, u64)>,
}

/// Sessions grouped by the day they *started*. A session that crosses midnight
/// counts towards the day it began, which is how a player thinks about it.
pub fn days(sessions: &[Session]) -> BTreeMap<String, DaySummary> {
    let mut out: BTreeMap<String, DaySummary> = BTreeMap::new();
    for session in sessions {
        let Some(date) = date_of(&session.started_at) else { continue };
        let entry = out.entry(date.clone()).or_insert_with(|| DaySummary {
            date,
            ..Default::default()
        });
        entry.seconds += session.seconds;
        entry.session_count += 1;
        match entry.games.iter_mut().find(|(name, _)| *name == session.game_name) {
            Some((_, seconds)) => *seconds += session.seconds,
            None => entry.games.push((session.game_name.clone(), session.seconds)),
        }
    }
    for day in out.values_mut() {
        day.games.sort_by(|a, b| b.1.cmp(&a.1));
    }
    out
}

/// Streaks, counted in whole days.
///
/// `today` is supplied so the caller decides what "now" means. A streak stays
/// alive if the last qualifying day was today *or* yesterday — a player who has
/// not started tonight's session yet has not lost their streak.
pub fn streaks(sessions: &[Session], threshold_minutes: u64, today: Date) -> StreakSummary {
    let by_day = days(sessions);
    let threshold = threshold_minutes * 60;

    let qualifying: Vec<Date> = by_day
        .values()
        .filter(|day| day.seconds >= threshold)
        .filter_map(|day| parse_date(&day.date))
        .collect();

    let total_seconds = by_day.values().map(|d| d.seconds).sum();
    let mut summary = StreakSummary {
        total_days: qualifying.len() as u64,
        total_seconds,
        best_month: best_month(&by_day),
        ..Default::default()
    };
    if qualifying.is_empty() {
        return summary;
    }

    // Longest run of consecutive days anywhere in the history.
    let mut longest = 1u64;
    let mut run = 1u64;
    for pair in qualifying.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if b - a == Duration::days(1) {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 1;
        }
    }
    summary.longest = longest;

    // The current streak counts backwards from the most recent qualifying day,
    // but only if that day is today or yesterday.
    let last = *qualifying.last().expect("checked non-empty");
    if today - last <= Duration::days(1) {
        let mut current = 1u64;
        let mut cursor = last;
        for day in qualifying.iter().rev().skip(1) {
            if cursor - *day == Duration::days(1) {
                current += 1;
                cursor = *day;
            } else {
                break;
            }
        }
        summary.current = current;

        let streak_start = cursor;
        let mut totals: BTreeMap<String, u64> = BTreeMap::new();
        for session in sessions {
            if let Some(date) = date_of(&session.started_at).and_then(|d| parse_date(&d)) {
                if date >= streak_start {
                    *totals.entry(session.game_name.clone()).or_default() += session.seconds;
                }
            }
        }
        let mut games: Vec<(String, u64)> = totals.into_iter().collect();
        games.sort_by(|a, b| b.1.cmp(&a.1));
        summary.current_streak_games = games.into_iter().map(|(name, _)| name).collect();
    }

    summary
}

fn best_month(by_day: &BTreeMap<String, DaySummary>) -> Option<(String, u64)> {
    let mut months: BTreeMap<String, u64> = BTreeMap::new();
    for (date, day) in by_day {
        if date.len() >= 7 {
            *months.entry(date[..7].to_string()).or_default() += day.seconds;
        }
    }
    months.into_iter().max_by_key(|(_, seconds)| *seconds)
}

/// The most recently played games, newest first, one entry per game.
pub fn recently_played(sessions: &[Session], limit: usize) -> Vec<(String, String, u64)> {
    let mut latest: BTreeMap<String, (String, String, u64)> = BTreeMap::new();
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    for session in sessions {
        *totals.entry(session.game_id.clone()).or_default() += session.seconds;
        let entry = latest
            .entry(session.game_id.clone())
            .or_insert_with(|| (session.game_name.clone(), session.ended_at.clone(), 0));
        if session.ended_at > entry.1 {
            entry.1 = session.ended_at.clone();
        }
    }
    let mut out: Vec<(String, String, u64)> = latest
        .into_iter()
        .map(|(id, (_name, ended, _))| (id.clone(), ended, totals.get(&id).copied().unwrap_or(0)))
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out.truncate(limit);
    out
}

/* -------------------------------------------------------------------------- */
/* Time helpers                                                               */
/* -------------------------------------------------------------------------- */

fn format_time(value: OffsetDateTime) -> String {
    value
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn parse_time(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).ok()
}

/// The date part of an RFC 3339 timestamp, without parsing the whole thing.
fn date_of(timestamp: &str) -> Option<String> {
    (timestamp.len() >= 10).then(|| timestamp[..10].to_string())
}

fn parse_date(value: &str) -> Option<Date> {
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    Date::from_calendar_date(year, time::Month::try_from(month).ok()?, day).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn game(id: &str, name: &str) -> (String, String) {
        (id.to_string(), name.to_string())
    }

    /// Feeds the watcher's ten-second ticks for `minutes`, returning what closed.
    fn play(activity: &mut Activity, running: &[(String, String)], from: OffsetDateTime, minutes: i64) -> OffsetDateTime {
        let mut now = from;
        for _ in 0..(minutes * 6) {
            observe(activity, running, now);
            now += Duration::seconds(10);
        }
        now
    }

    fn seen(id: &str, name: &str, active: bool) -> Running {
        Running { game_id: id.into(), game_name: name.into(), active }
    }

    #[test]
    fn a_session_opens_when_a_game_starts_and_closes_when_it_stops() {
        let mut activity = Activity::new();
        let start = datetime!(2026-08-30 20:00:00 UTC);

        let closed = observe(&mut activity, &[game("steam:440", "Team Fortress 2")], start);
        assert!(closed.is_empty());
        assert_eq!(activity.open.len(), 1);

        let end = play(&mut activity, &[game("steam:440", "Team Fortress 2")], start, 42);
        let closed = observe(&mut activity, &[], end);
        assert_eq!(closed.len(), 1);
        assert_eq!(activity.sessions.len(), 1);
        assert_eq!(activity.sessions[0].seconds, 42 * 60);
        assert_eq!(activity.sessions[0].wall_seconds, 42 * 60);
        assert!(activity.open.is_empty());
    }

    #[test]
    fn a_game_that_keeps_running_does_not_start_a_second_session() {
        let mut activity = Activity::new();
        let start = datetime!(2026-08-30 20:00:00 UTC);
        let running = [game("steam:440", "Team Fortress 2")];
        observe(&mut activity, &running, start);
        observe(&mut activity, &running, start + Duration::minutes(5));
        observe(&mut activity, &running, start + Duration::minutes(10));
        assert_eq!(activity.open.len(), 1);
        assert!(activity.sessions.is_empty());
    }

    #[test]
    fn alt_tabbing_through_a_launcher_does_not_litter_the_calendar() {
        let mut activity = Activity::new();
        let start = datetime!(2026-08-30 20:00:00 UTC);
        observe(&mut activity, &[game("steam:1", "Blink")], start);
        let closed = observe(&mut activity, &[], start + Duration::seconds(20));
        assert!(closed.is_empty());
        assert!(activity.sessions.is_empty(), "a 20-second run is not a session");
    }

    #[test]
    fn two_games_at_once_are_two_sessions() {
        let mut activity = Activity::new();
        let start = datetime!(2026-08-30 20:00:00 UTC);
        let end = play(&mut activity, &[game("steam:1", "A"), game("steam:2", "B")], start, 30);
        observe(&mut activity, &[], end);
        assert_eq!(activity.sessions.len(), 2);
    }

    #[test]
    fn disabling_tracking_closes_what_is_open_and_records_nothing_new() {
        let mut activity = Activity::new();
        let start = datetime!(2026-08-30 20:00:00 UTC);
        let end = play(&mut activity, &[game("steam:1", "A")], start, 30);

        activity.tracking_enabled = false;
        let closed = observe(&mut activity, &[game("steam:1", "A")], end);
        assert_eq!(closed.len(), 1, "the open session is finished rather than abandoned");

        observe(&mut activity, &[game("steam:1", "A")], start + Duration::minutes(60));
        assert_eq!(activity.sessions.len(), 1, "nothing new is recorded while tracking is off");
        assert!(activity.open.is_empty());
    }

    #[test]
    fn overnight_sessions_from_the_old_method_are_pruned_but_measured_ones_stay() {
        let mut activity = Activity::new();
        activity.sessions = vec![
            Session { game_id: "steam:1".into(), game_name: "Siege".into(), started_at: "2026-09-10T12:00:00Z".into(), ended_at: "2026-09-11T11:00:00Z".into(), seconds: 23 * 3600, wall_seconds: 0 },
            Session { game_id: "steam:1".into(), game_name: "Siege".into(), started_at: "2026-09-10T12:00:00Z".into(), ended_at: "2026-09-10T14:00:00Z".into(), seconds: 2 * 3600, wall_seconds: 0 },
            Session { game_id: "steam:1".into(), game_name: "Siege".into(), started_at: "2026-09-12T12:00:00Z".into(), ended_at: "2026-09-13T02:00:00Z".into(), seconds: 13 * 3600, wall_seconds: 14 * 3600 },
        ];
        assert_eq!(prune_impossible(&mut activity), 1);
        assert_eq!(activity.sessions.len(), 2);
        assert!(activity.sessions.iter().all(|s| s.seconds != 23 * 3600));
    }

    #[test]
    fn time_in_the_background_does_not_count() {
        let mut activity = Activity::new();
        let start = datetime!(2026-09-11 12:30:00 UTC);
        let mut now = start;
        // Ten minutes playing, twenty minutes alt-tabbed, ten minutes playing.
        for (minutes, active) in [(10, true), (20, false), (10, true)] {
            for _ in 0..(minutes * 6) {
                observe_active(&mut activity, &[seen("steam:359550", "Siege", active)], now);
                now += Duration::seconds(10);
            }
        }
        let closed = observe_active(&mut activity, &[], now);
        assert_eq!(closed.len(), 1);
        let session = &activity.sessions[0];
        // Each switch loses at most one ten-second tick; that is the resolution.
        assert!((session.seconds as i64 - 20 * 60).abs() <= 20, "only the active twenty minutes count, got {}", session.seconds);
        assert_eq!(session.wall_seconds, 40 * 60, "but the session remembers it was open for forty");
    }

    #[test]
    fn a_sleeping_pc_adds_nothing() {
        let mut activity = Activity::new();
        let start = datetime!(2026-09-11 12:30:00 UTC);
        let mut now = play(&mut activity, &[game("steam:1", "A")], start, 5);
        // The lid closes for three hours; the next tick is far away.
        now += Duration::hours(3);
        observe(&mut activity, &[game("steam:1", "A")], now);
        now = play(&mut activity, &[game("steam:1", "A")], now, 5);
        observe(&mut activity, &[], now);
        assert!((activity.sessions[0].seconds as i64 - 10 * 60).abs() <= 20, "got {}", activity.sessions[0].seconds);
    }

    #[test]
    fn a_game_only_ever_in_the_background_is_not_a_session() {
        let mut activity = Activity::new();
        let start = datetime!(2026-09-11 12:30:00 UTC);
        let mut now = start;
        for _ in 0..(30 * 6) {
            observe_active(&mut activity, &[seen("epic:Fortnite", "Fortnite", false)], now);
            now += Duration::seconds(10);
        }
        let closed = observe_active(&mut activity, &[], now);
        assert!(closed.is_empty(), "a launcher left open in the background is not play");
    }

    #[test]
    fn the_live_counter_includes_the_current_tick_only_while_active() {
        let mut activity = Activity::new();
        let start = datetime!(2026-09-11 12:30:00 UTC);
        let now = play(&mut activity, &[game("steam:1", "A")], start, 2);
        let open = &activity.open[0];
        assert_eq!(active_so_far(open, now + Duration::seconds(5)), 120 + 5);
        let mut idle = open.clone();
        idle.active = false;
        // 12 ticks make 11 counted intervals; the part-tick is not added while idle.
        assert_eq!(active_so_far(&idle, now + Duration::seconds(5)), 110);
    }

    /// Builds a session on a given day, of a given length.
    fn session(date: &str, name: &str, minutes: u64) -> Session {
        Session {
            game_id: format!("steam:{name}"),
            game_name: name.to_string(),
            started_at: format!("{date}T20:00:00Z"),
            ended_at: format!("{date}T21:00:00Z"),
            seconds: minutes * 60,
            wall_seconds: minutes * 60,
        }
    }

    #[test]
    fn days_group_sessions_and_rank_games_by_time() {
        let sessions = vec![
            session("2026-08-30", "Minecraft", 130),
            session("2026-08-30", "Fortnite", 71),
            session("2026-08-29", "Minecraft", 45),
        ];
        let by_day = days(&sessions);
        assert_eq!(by_day.len(), 2);
        let today = &by_day["2026-08-30"];
        assert_eq!(today.seconds, 201 * 60);
        assert_eq!(today.session_count, 2);
        assert_eq!(today.games[0].0, "Minecraft", "the most played game comes first");
    }

    #[test]
    fn a_streak_counts_consecutive_days_over_the_threshold() {
        let sessions = vec![
            session("2026-08-28", "A", 30),
            session("2026-08-29", "A", 30),
            session("2026-08-30", "A", 30),
        ];
        let summary = streaks(&sessions, 15, parse_date("2026-08-30").unwrap());
        assert_eq!(summary.current, 3);
        assert_eq!(summary.longest, 3);
        assert_eq!(summary.total_days, 3);
    }

    #[test]
    fn a_day_under_the_threshold_breaks_the_streak() {
        let sessions = vec![
            session("2026-08-28", "A", 30),
            // Five minutes does not count as a gaming day.
            session("2026-08-29", "A", 5),
            session("2026-08-30", "A", 30),
        ];
        let summary = streaks(&sessions, 15, parse_date("2026-08-30").unwrap());
        assert_eq!(summary.current, 1);
        assert_eq!(summary.total_days, 2);
    }

    #[test]
    fn not_having_played_yet_today_does_not_lose_yesterdays_streak() {
        let sessions = vec![session("2026-08-28", "A", 30), session("2026-08-29", "A", 30)];
        let summary = streaks(&sessions, 15, parse_date("2026-08-30").unwrap());
        assert_eq!(summary.current, 2, "the streak survives until a whole day is missed");

        let summary = streaks(&sessions, 15, parse_date("2026-08-31").unwrap());
        assert_eq!(summary.current, 0, "two days later it is gone");
        assert_eq!(summary.longest, 2, "but the record stands");
    }

    #[test]
    fn the_streak_threshold_is_configurable() {
        let sessions = vec![session("2026-08-30", "A", 20)];
        assert_eq!(streaks(&sessions, 15, parse_date("2026-08-30").unwrap()).current, 1);
        assert_eq!(streaks(&sessions, 30, parse_date("2026-08-30").unwrap()).current, 0);
    }

    #[test]
    fn the_longest_streak_is_found_anywhere_in_the_history() {
        let mut sessions = Vec::new();
        for day in 1..=5 {
            sessions.push(session(&format!("2026-06-0{day}"), "A", 60));
        }
        sessions.push(session("2026-08-30", "A", 60));
        let summary = streaks(&sessions, 15, parse_date("2026-08-30").unwrap());
        assert_eq!(summary.longest, 5);
        assert_eq!(summary.current, 1);
    }

    #[test]
    fn best_month_is_the_one_with_the_most_hours() {
        let sessions = vec![
            session("2026-06-01", "A", 600),
            session("2026-08-30", "A", 60),
        ];
        let summary = streaks(&sessions, 15, parse_date("2026-08-30").unwrap());
        assert_eq!(summary.best_month.unwrap().0, "2026-06");
    }

    #[test]
    fn recently_played_lists_each_game_once_newest_first() {
        let sessions = vec![
            session("2026-08-28", "Old", 60),
            session("2026-08-29", "New", 60),
            session("2026-08-30", "New", 60),
        ];
        let recent = recently_played(&sessions, 10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].0, "steam:New");
        assert_eq!(recent[0].2, 120 * 60, "playtime is the total across sessions");
    }

    #[test]
    fn a_session_crossing_midnight_belongs_to_the_day_it_started() {
        let sessions = vec![Session {
            game_id: "steam:1".into(),
            game_name: "Night".into(),
            started_at: "2026-08-30T23:30:00Z".into(),
            ended_at: "2026-08-31T01:30:00Z".into(),
            seconds: 2 * 3600,
            wall_seconds: 2 * 3600,
        }];
        let by_day = days(&sessions);
        assert!(by_day.contains_key("2026-08-30"));
        assert!(!by_day.contains_key("2026-08-31"));
    }
}
