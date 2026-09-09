//! Detection primitives shared by every GameHub launcher adapter.

pub mod activity;
pub mod backup;
pub mod clipboard;
pub mod env;
pub mod error;
pub mod library;
pub mod media;
pub mod model;
pub mod quests;
pub mod safepath;
pub mod saves;
pub mod freeze;
pub mod shortcuts;
pub mod vdf;

pub use activity::{Activity, DaySummary, Session, StreakSummary};
pub use quests::{Quest, QuestBoard, QuestPeriod};
pub use env::{Env, FakeRegistry, Hive, KnownFolders, RegistryReader};
pub use error::{DetectError, Result};
pub use model::{Game, GameMetadata, GameSource, LaunchMethod, LauncherStatus, ScanResult};

use time::format_description::well_known::Rfc3339;

/// One timestamp format everywhere: RFC 3339, UTC. The TypeScript side parses
/// these with `new Date(...)` without any special handling.
pub fn now_iso8601() -> String {
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

/// Unix seconds to RFC 3339. Steam and GOG both store times this way.
pub fn unix_to_iso8601(seconds: i64) -> Option<String> {
    time::OffsetDateTime::from_unix_timestamp(seconds)
        .ok()?
        .format(&Rfc3339)
        .ok()
}
