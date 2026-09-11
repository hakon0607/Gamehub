/**
 * The whole surface between the interface and the machine.
 *
 * Every function here is a Tauri command. Note what is missing: nothing takes a
 * path to run or a command line. The UI sends ids; the Rust side decides what
 * that means and validates it before anything starts.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Game, ScanResult } from '@gamehub/shared';

export interface Settings {
  startWithWindows: boolean;
  minimiseToTray: boolean;
  scanIntervalMinutes: number;
  autoAddNewGames: boolean;
  trackActivity: boolean;
  trackActiveOnly: boolean;
  activeOnlyChosen: boolean;
  idleMinutes: number;
  streakThresholdMinutes: number;
  screenshotFolder: string;
  screenshotMonitor: number;
  clipboardEnabled: boolean;
  accent: string;
  density: string;
  backgroundImage: string;
  replay: ReplaySettingsShape;
  extraGameFolders: string[];
  metadata: { igdbClientId: string; igdbClientSecret: string; steamGridDbKey: string };
  ai: { enabled: boolean; provider: string; apiKey: string; model: string };
  cloud: { enabled: boolean; apiUrl: string };
  language: string;
  /** False until the language screen has been answered once. */
  languageChosen: boolean;
  overlayPopup: boolean;
  overlaySound: boolean;
  wallpapers: { desktop: string; lock: string; monitors: Record<string, string> };
  theme: string;
  onboarded: boolean;
  dismissedUpdateVersion: string | null;
}

/* -------------------------------------------------------------------------- */
/* Activity                                                                   */
/* -------------------------------------------------------------------------- */

export interface Session {
  gameId: string;
  gameName: string;
  startedAt: string;
  endedAt: string;
  /** Active play — the game in front, the player at the keyboard. */
  seconds: number;
  /** How long the game was open in total. 0 on sessions from older versions. */
  wallSeconds: number;
}

export interface DaySummary {
  date: string;
  seconds: number;
  /** Game name and seconds, most played first. */
  games: [string, number][];
  sessionCount: number;
}

export interface StreakSummary {
  current: number;
  longest: number;
  totalDays: number;
  totalSeconds: number;
  currentStreakGames: string[];
  bestMonth: [string, number] | null;
}

export interface ActivitySummary {
  streaks: StreakSummary;
  today: DaySummary;
  /** Game id, when it was last played, total seconds. */
  recent: [string, string, number][];
  current: Session | null;
  /** The current game is being played right now, not sitting in the background. */
  currentActive: boolean;
  trackingEnabled: boolean;
}

/* -------------------------------------------------------------------------- */
/* Screenshots, clipboard, performance, shortcuts                             */
/* -------------------------------------------------------------------------- */

export interface Screenshot {
  id: string;
  path: string;
  gameId: string | null;
  gameName: string;
  takenAt: string;
  sizeBytes: number;
  favorite: boolean;
}

export interface ClipItem {
  id: string;
  text: string;
  length: number;
  truncated: boolean;
  copiedAt: string;
  pinned: boolean;
  isUrl: boolean;
}

export interface GameProcess {
  name: string;
  cpuPercent: number;
  memoryBytes: number;
}

export interface DiskUsage {
  name: string;
  usedBytes: number;
  totalBytes: number;
}

export interface PerformanceSample {
  cpuPercent: number;
  cpuName: string;
  cpuCores: number;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  swapUsedBytes: number;
  diskReadBytes: number;
  diskWrittenBytes: number;
  networkDownBytes: number;
  networkUpBytes: number;
  disks: DiskUsage[];
  gameProcesses: GameProcess[];
  /** Always null — see unavailableNote. */
  fps: number | null;
  gpuPercent: number | null;
  cpuTemperatureC: number | null;
  unavailableNote: string;
  sampledAt: string;
}

export interface Clip {
  id: string;
  path: string;
  gameId: string | null;
  gameName: string;
  recordedAt: string;
  seconds: number;
  sizeBytes: number;
  favorite: boolean;
  /** False for clips made with every sound source off. */
  hasAudio: boolean;
}

export interface ReplaySettingsShape {
  enabled: boolean;
  /** How many seconds the hotkey saves. 30 by default. */
  saveSeconds: number;
  /** Record at this height instead of the desktop's own. 0 means native. */
  scaleHeight: number;
  /** "auto" for the graphics card's encoder, "cpu" to force libx264. */
  encoder: string;
  /** 30–600 seconds of history. */
  bufferSeconds: number;
  fps: number;
  quality: string;
  monitor: number;
  /** What comes out of the speakers, via WASAPI loopback. */
  systemAudio: boolean;
  /** A microphone to mix in, or empty. */
  audioDevice: string;
  folder: string;
}

export interface AudioOptions {
  devices: string[];
  /** A device that captures what you hear, when one is listed as a microphone. */
  suggested: string | null;
}

export interface AudioReport {
  systemAudio: boolean;
  systemDevice: string | null;
  microphone: boolean;
  /** Why system audio is off although it was asked for. */
  note: string | null;
}

export interface ReplayStatus {
  enabled: boolean;
  running: boolean;
  /** Why recording stopped, when it is on but not running. */
  problem: string | null;
  /** Null when ffmpeg is missing — the one thing that stops replay working. */
  ffmpegPath: string | null;
  bufferSeconds: number;
  /** How many seconds are actually recorded right now. */
  bufferedSeconds: number;
  minBufferSeconds: number;
  maxBufferSeconds: number;
  clipFolder: string;
  bufferEstimateMb: number;
  audio: AudioReport;
}

/* -------------------------------------------------------------------------- */
/* Freeze points                                                              */
/* -------------------------------------------------------------------------- */

export type FreezeState = 'frozen' | 'resumed' | 'gone';

export interface FreezePoint {
  id: string;
  gameId: string | null;
  gameName: string;
  frozenAt: string;
  state: FreezeState;
  pids: number[];
  folder: string;
  saveDir: string | null;
  saveCopy: string | null;
  saveFiles: number;
  saveBytes: number;
  skipped: string[];
  picture: string | null;
  note: string;
}

export interface FreezeStatus {
  supported: boolean;
  currentGameId: string | null;
  currentGameName: string | null;
  active: FreezePoint | null;
  saveDir: string | null;
  folder: string;
}

export type WallpaperTarget = 'desktop' | 'lock';

export interface WallpaperPicture {
  path: string;
  fileName: string;
  width: number;
  height: number;
  bytes: number;
}

export interface WallpaperMonitor {
  id: string;
  index: number;
  width: number;
  height: number;
  /** What Windows shows on this monitor right now. */
  current: string;
  chosen: WallpaperPicture | null;
}

export interface WallpaperStatus {
  supported: boolean;
  lockSupported: boolean;
  desktop: WallpaperPicture | null;
  desktopActive: boolean;
  lock: WallpaperPicture | null;
  monitors: WallpaperMonitor[];
  folder: string;
}

export interface SaveCandidate {
  path: string;
  location: string;
  files: number;
  bytes: number;
  modifiedAt: string | null;
  confidence: number;
}

export interface DataFolders {
  data: string;
  /** Beside the data folder, so an uninstall cannot take the backups too. */
  backups: string;
}

export interface Backup {
  id: string;
  reason: string;
  appVersion: string;
  takenAt: string;
  files: string[];
  bytes: number;
}

export interface RecoveryNote {
  file: string;
  /** Where the unreadable original was kept — nothing is ever deleted. */
  keptAt: string;
  restored: boolean;
  reason: string;
}

export interface ShortcutEntry {
  action: string;
  label: string;
  scope: 'global' | 'app';
  binding: string;
  defaultBinding: string;
}

/* -------------------------------------------------------------------------- */
/* Quests                                                                     */
/* -------------------------------------------------------------------------- */

export type QuestPeriod = 'daily' | 'biweekly' | 'monthly';

/** What a quest asks for; the interface puts it into words in its own language. */
export type QuestGoal =
  | { kind: 'playGame'; gameId: string; gameName: string; seconds: number }
  | { kind: 'distinctGames'; count: number }
  | { kind: 'playOnDays'; days: number }
  | { kind: 'totalPlaytime'; seconds: number }
  | { kind: 'revisit'; gameId: string; gameName: string; seconds: number }
  | { kind: 'trySomethingNew'; seconds: number };

export interface Quest {
  id: string;
  period: QuestPeriod;
  /** English, from the generator. The interface uses `goal` instead. */
  title: string;
  description: string;
  goal: QuestGoal;
  xp: number;
  progress: number;
  target: number;
  complete: boolean;
}

export interface QuestBoard {
  daily: Quest[];
  biweekly: Quest[];
  monthly: Quest[];
  xp: number;
  level: number;
  xpIntoLevel: number;
  xpForLevel: number;
}

export const api = {
  getLibrary: () => invoke<Game[]>('get_library'),
  getRunningGames: () => invoke<string[]>('get_running_games'),
  scanNow: () => invoke<ScanResult>('scan_now'),
  launchGame: (gameId: string) => invoke<void>('launch_game', { gameId }),
  setFavorite: (gameId: string, favorite: boolean) => invoke<void>('set_favorite', { gameId, favorite }),
  setHidden: (gameId: string, hidden: boolean) => invoke<void>('set_hidden', { gameId, hidden }),
  renameGame: (gameId: string, name: string) => invoke<void>('rename_game', { gameId, name }),
  setTags: (gameId: string, tags: string[]) => invoke<void>('set_tags', { gameId, tags }),
  openGameFolder: (gameId: string) => invoke<string>('open_game_folder', { gameId }),

  getSettings: () => invoke<Settings>('get_settings'),
  saveSettings: (settings: Settings) => invoke<Settings>('save_settings', { settings }),
  addGameFolder: (folder: string) => invoke<Settings>('add_game_folder', { folder }),
  addManualGame: (executable: string, name?: string) =>
    invoke<Game>('add_manual_game', { executable, name: name ?? null }),
  removeGame: (gameId: string) => invoke<string>('remove_game', { gameId }),
  restoreHidden: () => invoke<number>('restore_hidden'),
  hiddenCount: () => invoke<number>('hidden_count'),

  refreshArtwork: () => invoke<void>('refresh_artwork'),
  readImageForCrop: (path: string) => invoke<string>('read_image_for_crop', { path }),
  setCustomCover: (gameId: string, pngDataUrl: string) =>
    invoke<string>('set_custom_cover', { gameId, pngDataUrl }),
  clearCustomCover: (gameId: string) => invoke<void>('clear_custom_cover', { gameId }),
  getActivity: () => invoke<ActivitySummary>('get_activity'),
  getQuests: () => invoke<QuestBoard>('get_quests'),
  getCalendarMonth: (month: string) => invoke<DaySummary[]>('get_calendar_month', { month }),
  clearActivity: () => invoke<void>('clear_activity'),

  takeScreenshot: () => invoke<Screenshot>('take_screenshot'),
  getScreenshots: () => invoke<[string, Screenshot[]][]>('get_screenshots'),
  setScreenshotFavorite: (id: string, favorite: boolean) =>
    invoke<void>('set_screenshot_favorite', { id, favorite }),
  deleteScreenshot: (id: string) => invoke<void>('delete_screenshot', { id }),
  screenshotFolder: () => invoke<string>('screenshot_folder'),
  listDisplays: () => invoke<string[]>('list_displays'),

  getClipboard: (query?: string) => invoke<ClipItem[]>('get_clipboard', { query: query ?? null }),
  pinClip: (id: string, pinned: boolean) => invoke<void>('pin_clip', { id, pinned }),
  deleteClip: (id: string) => invoke<void>('delete_clip', { id }),
  clearClipboard: () => invoke<void>('clear_clipboard'),

  samplePerformance: () => invoke<PerformanceSample>('sample_performance'),

  replayStatus: () => invoke<ReplayStatus>('replay_status'),
  setReplayEnabled: (enabled: boolean) => invoke<void>('set_replay_enabled', { enabled }),
  saveReplay: (seconds?: number) => invoke<Clip>('save_replay', { seconds: seconds ?? null }),
  getClips: () => invoke<Clip[]>('get_clips'),
  trimClip: (id: string, start: number, end: number, replace: boolean) =>
    invoke<Clip>('trim_clip', { id, start, end, replace }),
  deleteReplayClip: (id: string) => invoke<void>('delete_replay_clip', { id }),
  setClipFavorite: (id: string, favorite: boolean) => invoke<void>('set_clip_favorite', { id, favorite }),

  getShortcuts: () => invoke<ShortcutEntry[]>('get_shortcuts'),

  // Your data: backups, and what had to be recovered on the way in.
  listBackups: () => invoke<Backup[]>('list_backups'),
  getRecoveries: () => invoke<RecoveryNote[]>('get_recoveries'),
  backupNow: () => invoke<Backup | null>('backup_now'),
  restoreBackup: (id: string) => invoke<string[]>('restore_backup', { id }),
  dataFolder: () => invoke<DataFolders>('data_folder'),
  updatesSupported: () => invoke<boolean>('updates_supported'),
  replayAudioDevices: () => invoke<AudioOptions>('replay_audio_devices'),
  setShortcut: (action: string, binding: string) => invoke<string>('set_shortcut', { action, binding }),
  resetShortcut: (action: string) => invoke<void>('reset_shortcut', { action }),

  // Freeze points.
  freezeStatus: () => invoke<FreezeStatus>('freeze_status'),
  wallpaperStatus: () => invoke<WallpaperStatus>('wallpaper_status'),
  setWallpaper: (target: WallpaperTarget, path: string, monitor?: string) =>
    invoke<WallpaperStatus>('set_wallpaper', { target, path, monitor: monitor ?? null }),
  forgetWallpaper: (target: WallpaperTarget, monitor?: string) =>
    invoke<WallpaperStatus>('forget_wallpaper', { target, monitor: monitor ?? null }),
  defaultWallpaper: (target: WallpaperTarget, monitor?: string) =>
    invoke<WallpaperStatus>('default_wallpaper', { target, monitor: monitor ?? null }),
  getFreezes: () => invoke<FreezePoint[]>('get_freezes'),
  freezeNow: (gameId?: string) => invoke<FreezePoint>('freeze_now', { gameId: gameId ?? null }),
  resumeFreeze: (id: string) => invoke<FreezePoint>('resume_freeze', { id }),
  restoreFreezeSave: (id: string) => invoke<string>('restore_freeze_save', { id }),
  deleteFreeze: (id: string) => invoke<void>('delete_freeze', { id }),
  setFreezeNote: (id: string, note: string) => invoke<void>('set_freeze_note', { id, note }),
  findSaveFolders: (gameId: string) => invoke<SaveCandidate[]>('find_save_folders', { gameId }),
  setSaveFolder: (gameId: string, folder: string) => invoke<void>('set_save_folder', { gameId, folder }),
  getSaveFolder: (gameId: string) => invoke<string | null>('get_save_folder', { gameId }),
};

export const events = {
  onLibraryUpdated: (fn: (result: ScanResult) => void): Promise<UnlistenFn> =>
    listen<ScanResult>('library-updated', (e) => fn(e.payload)),
  onGamesDiscovered: (fn: (games: Game[]) => void): Promise<UnlistenFn> =>
    listen<Game[]>('games-discovered', (e) => fn(e.payload)),
  onRunningGames: (fn: (ids: string[]) => void): Promise<UnlistenFn> =>
    listen<string[]>('running-games', (e) => fn(e.payload)),
  onArtworkUpdated: (fn: (count: number) => void): Promise<UnlistenFn> =>
    listen<number>('artwork-updated', (e) => fn(e.payload)),
  onActivityUpdated: (fn: () => void): Promise<UnlistenFn> =>
    listen('activity-updated', () => fn()),
  onScreenshotTaken: (fn: (shot: Screenshot) => void): Promise<UnlistenFn> =>
    listen<Screenshot>('screenshot-taken', (e) => fn(e.payload)),
  onClipboardUpdated: (fn: () => void): Promise<UnlistenFn> =>
    listen('clipboard-updated', () => fn()),
  onClipSaved: (fn: (clip: Clip) => void): Promise<UnlistenFn> =>
    listen<Clip>('clip-saved', (e) => fn(e.payload)),
  onLibraryChanged: (fn: () => void): Promise<UnlistenFn> => listen('library-changed', () => fn()),
  onToast: (fn: (title: string, body: string) => void): Promise<UnlistenFn> =>
    listen<[string, string]>('toast', (e) => fn(e.payload[0], e.payload[1])),
  onShortcutUnavailable: (fn: (action: string, binding: string) => void): Promise<UnlistenFn> =>
    listen<[string, string]>('shortcut-unavailable', (e) => fn(e.payload[0], e.payload[1])),
  onFreezeChanged: (fn: () => void): Promise<UnlistenFn> => listen('freeze-changed', () => fn()),
  onReplayState: (fn: (on: boolean) => void): Promise<UnlistenFn> =>
    listen<boolean>('replay-state', (e) => fn(e.payload)),
  onSettingsUpdated: (fn: (settings: Settings) => void): Promise<UnlistenFn> =>
    listen<Settings>('settings-updated', (e) => fn(e.payload)),
  onToggleOverlay: (fn: () => void): Promise<UnlistenFn> => listen('toggle-overlay', () => fn()),
};
