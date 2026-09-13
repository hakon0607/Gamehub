//! The library model. Mirrors `packages/shared/src/index.ts` exactly — serde
//! renames to camelCase so the TypeScript types are the same types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameSource {
    Steam,
    Epic,
    Xbox,
    Ea,
    Ubisoft,
    Battlenet,
    Gog,
    Riot,
    Local,
}

impl GameSource {
    pub fn slug(self) -> &'static str {
        match self {
            GameSource::Steam => "steam",
            GameSource::Epic => "epic",
            GameSource::Xbox => "xbox",
            GameSource::Ea => "ea",
            GameSource::Ubisoft => "ubisoft",
            GameSource::Battlenet => "battlenet",
            GameSource::Gog => "gog",
            GameSource::Riot => "riot",
            GameSource::Local => "local",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GameSource::Steam => "Steam",
            GameSource::Epic => "Epic Games",
            GameSource::Xbox => "Xbox",
            GameSource::Ea => "EA",
            GameSource::Ubisoft => "Ubisoft Connect",
            GameSource::Battlenet => "Battle.net",
            GameSource::Gog => "GOG",
            GameSource::Riot => "Riot Games",
            GameSource::Local => "Local",
        }
    }
}

/// How a game is started.
///
/// The UI never supplies one of these. It sends a game id; the backend looks up
/// the method it recorded when the game was discovered, re-validates it, and
/// only then starts anything. That is what keeps "launch a game" from becoming
/// "run an arbitrary command".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchMethod {
    /// A launcher's own protocol handler. Always preferred: the launcher does
    /// the DRM handshake, the update check and the overlay, and GameHub never
    /// touches an executable.
    Uri { uri: String },
    #[serde(rename_all = "camelCase")]
    Executable {
        path: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Uwp { app_user_model_id: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadata {
    pub cover_path: Option<String>,
    pub hero_path: Option<String>,
    pub logo_path: Option<String>,
    pub description: Option<String>,
    pub genres: Vec<String>,
    pub release_date: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub provider: Option<String>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub source: GameSource,
    pub source_id: String,
    pub name: String,
    pub install_dir: Option<String>,
    pub installed: bool,
    pub launch: Option<LaunchMethod>,
    pub size_bytes: Option<u64>,
    pub playtime_seconds: Option<u64>,
    pub last_played: Option<String>,
    pub favorite: bool,
    pub hidden: bool,
    pub tags: Vec<String>,
    pub metadata: Option<GameMetadata>,
    pub discovered_at: String,
}

impl Game {
    /// The id is derived, never random, so the same game keeps its favourites
    /// and tags across rescans, reinstalls and app restarts.
    pub fn make_id(source: GameSource, source_id: &str) -> String {
        format!("{}:{}", source.slug(), source_id)
    }

    pub fn new(source: GameSource, source_id: impl Into<String>, name: impl Into<String>) -> Self {
        let source_id = source_id.into();
        Self {
            id: Self::make_id(source, &source_id),
            source,
            source_id,
            name: name.into(),
            install_dir: None,
            installed: true,
            launch: None,
            size_bytes: None,
            playtime_seconds: None,
            last_played: None,
            favorite: false,
            hidden: false,
            tags: Vec::new(),
            metadata: None,
            discovered_at: crate::now_iso8601(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    pub source: GameSource,
    pub detected: bool,
    pub install_dir: Option<String>,
    pub game_count: usize,
    /// Set when an adapter could only do part of the job. Shown in Settings
    /// rather than hidden, so nothing pretends to work that does not.
    pub limitation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub launchers: Vec<LauncherStatus>,
    pub games: Vec<Game>,
    pub new_game_ids: Vec<String>,
    pub scanned_at: String,
    pub duration_ms: u64,
}
