/**
 * Types shared by the desktop app, the web app and the sync API.
 *
 * The Rust side serialises to exactly these shapes (serde renames to camelCase),
 * so a change here is a change there — see `src-tauri/src/model.rs`.
 */

/** Every launcher GameHub knows how to read. */
export const GAME_SOURCES = [
  'steam',
  'epic',
  'xbox',
  'ea',
  'ubisoft',
  'battlenet',
  'gog',
  'riot',
  'local',
] as const;
export type GameSource = (typeof GAME_SOURCES)[number];

export const SOURCE_LABELS: Record<GameSource, string> = {
  steam: 'Steam',
  epic: 'Epic Games',
  xbox: 'Xbox',
  ea: 'EA',
  ubisoft: 'Ubisoft Connect',
  battlenet: 'Battle.net',
  gog: 'GOG',
  riot: 'Riot Games',
  local: 'Local',
};

/**
 * How a game is started. The UI never sends a command line — it sends a game
 * id, and the Rust side turns that into one of these, which it validated when
 * the game was discovered.
 */
export type LaunchMethod =
  /** A launcher's own URI scheme, e.g. steam://rungameid/440. Always preferred. */
  | { kind: 'uri'; uri: string }
  /** An executable that was found inside a known install directory. */
  | { kind: 'executable'; path: string; args: string[]; workingDir: string | null }
  /** A Windows Store / UWP app, started through the shell's AppsFolder. */
  | { kind: 'uwp'; appUserModelId: string };

export interface Game {
  /** Stable across rescans: `${source}:${sourceId}`. */
  id: string;
  source: GameSource;
  /** The launcher's own id — Steam appid, Epic AppName, Ubisoft install id… */
  sourceId: string;
  name: string;
  installDir: string | null;
  installed: boolean;
  launch: LaunchMethod | null;
  sizeBytes: number | null;
  /** Seconds. Only some launchers expose this. */
  playtimeSeconds: number | null;
  lastPlayed: string | null;
  favorite: boolean;
  hidden: boolean;
  tags: string[];
  metadata: GameMetadata | null;
  /** When GameHub first saw it — drives the "new game detected" toast. */
  discoveredAt: string;
}

export interface GameMetadata {
  coverPath: string | null;
  heroPath: string | null;
  logoPath: string | null;
  description: string | null;
  genres: string[];
  releaseDate: string | null;
  developer: string | null;
  publisher: string | null;
  /**
   * Which provider filled this in, so a bad match can be re-fetched.
   * `steam-guess` means the title was matched by name rather than by id — the
   * UI marks those so a wrong cover is obvious rather than puzzling.
   */
  provider: 'steam' | 'steam-guess' | 'igdb' | 'steamgriddb' | 'manual' | null;
  fetchedAt: string | null;
}

export interface LauncherStatus {
  source: GameSource;
  /** False means the launcher is not installed on this machine. */
  detected: boolean;
  installDir: string | null;
  gameCount: number;
  /** Set when detection worked partially or not at all — shown in Settings. */
  limitation: string | null;
}

export interface ScanResult {
  launchers: LauncherStatus[];
  games: Game[];
  /** Ids that were not in the library before this scan. */
  newGameIds: string[];
  scannedAt: string;
  durationMs: number;
}

/* -------------------------------------------------------------------------- */
/* Settings and sync                                                          */
/* -------------------------------------------------------------------------- */

export interface Settings {
  startWithWindows: boolean;
  minimiseToTray: boolean;
  /** Minutes between background rescans. 0 disables the timer (watchers still run). */
  scanIntervalMinutes: number;
  autoAddNewGames: boolean;
  extraGameFolders: string[];
  metadata: {
    /** Both are the user's own free keys; empty means that provider is off. */
    igdbClientId: string;
    igdbClientSecret: string;
    steamGridDbKey: string;
  };
  ai: {
    enabled: boolean;
    provider: 'gemini' | 'groq' | 'openai' | 'openrouter' | 'ollama';
    apiKey: string;
    model: string;
  };
  cloud: {
    enabled: boolean;
    apiUrl: string;
  };
  language: 'en' | 'nb';
  theme: 'dark' | 'light' | 'system';
}

/** What a signed-in user syncs. Deliberately: no paths, no files, no credentials. */
export interface SyncPayload {
  favorites: string[];
  hidden: string[];
  tags: Record<string, string[]>;
  playHistory: { gameId: string; lastPlayed: string; playtimeSeconds: number }[];
  settings: Omit<Settings, 'metadata' | 'ai' | 'extraGameFolders'>;
  updatedAt: string;
}
