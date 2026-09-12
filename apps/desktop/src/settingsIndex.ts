import { t, type Key } from './i18n';

/**
 * Every setting GameHub has, in one list.
 *
 * Two things read it: the search box at the top of Settings, and the command
 * palette (Ctrl+K), so typing "lyd" anywhere in the app lands on the audio
 * setting rather than a page it might be on. A setting that is not in this
 * list cannot be found by search, which is the point — adding one here is what
 * makes it findable.
 */

export type SettingsCategory =
  | 'general'
  | 'appearance'
  | 'folders'
  | 'shortcuts'
  | 'replay'
  | 'freeze'
  | 'screenshots'
  | 'privacy'
  | 'data'
  | 'updates'
  | 'artwork'
  | 'assistant';

export interface SettingEntry {
  id: string;
  category: SettingsCategory;
  /** The translated title, from the `si.*` keys. */
  title: string;
  /** Extra words people might type, in English — titles are searched in
   * every language, so only synonyms need to live here. */
  keywords: string;
}

export interface Category {
  id: SettingsCategory;
  label: string;
  icon: string;
  blurb: string;
}

const CATEGORY_LIST: { id: SettingsCategory; icon: string }[] = [
  { id: 'general', icon: '⚙' },
  { id: 'appearance', icon: '◐' },
  { id: 'replay', icon: '⏺' },
  { id: 'freeze', icon: '❄' },
  { id: 'shortcuts', icon: '⌨' },
  { id: 'screenshots', icon: '⎙' },
  { id: 'folders', icon: '▤' },
  { id: 'privacy', icon: '◈' },
  { id: 'data', icon: '⛁' },
  { id: 'updates', icon: '↑' },
  { id: 'artwork', icon: '▣' },
  { id: 'assistant', icon: '✦' },
];

/** The categories, in the current language. Called at render time. */
export function categories(replayKey = 'F8'): Category[] {
  return CATEGORY_LIST.map(({ id, icon }) => ({
    id,
    icon,
    label: t(`settings.cat_${id}` as Key),
    blurb: t(`settings.cat_${id}_blurb` as Key, { key: replayKey }),
  }));
}

const ENTRIES: { id: string; category: SettingsCategory; keywords: string }[] = [
  { id: 'start-with-windows', category: 'general', keywords: 'startup autostart boot login' },
  { id: 'minimise-to-tray', category: 'general', keywords: 'tray close minimise background' },
  { id: 'scan-interval', category: 'general', keywords: 'scan interval new games automatic' },
  { id: 'auto-add', category: 'general', keywords: 'auto add new games discover' },
  { id: 'language', category: 'general', keywords: 'language språk sprache langue idioma taal kieli język' },
  { id: 'overlay-popup', category: 'general', keywords: 'popup overlay notification toast game hidden fullscreen varsel' },
  { id: 'overlay-sound', category: 'general', keywords: 'sound pling beep notification lyd' },
  { id: 'theme', category: 'appearance', keywords: 'theme colour dark blue black look' },
  { id: 'accent', category: 'appearance', keywords: 'accent colour color' },
  { id: 'density', category: 'appearance', keywords: 'compact spacing density' },
  { id: 'background', category: 'appearance', keywords: 'background wallpaper image' },
  { id: 'startup-animation', category: 'appearance', keywords: 'startup splash animation intro logo open launch oppstart' },
  { id: 'startup-on-reopen', category: 'appearance', keywords: 'startup animation tray reopen every time open again hver gang' },
  { id: 'startup-sound', category: 'appearance', keywords: 'startup sound chime intro open launch lyd oppstart' },
  { id: 'wallpaper-desktop', category: 'appearance', keywords: 'wallpaper desktop background windows bakgrunnsbilde skrivebord monitor screen' },
  { id: 'wallpaper-lock', category: 'appearance', keywords: 'wallpaper lock screen låseskjerm windows picture' },
  { id: 'replay-enabled', category: 'replay', keywords: 'replay record buffer on off' },
  { id: 'replay-buffer', category: 'replay', keywords: 'buffer length minutes seconds remember 10 min' },
  { id: 'replay-save', category: 'replay', keywords: 'f8 save clip length seconds' },
  { id: 'replay-system-audio', category: 'replay', keywords: 'sound audio speakers game sound loopback lyd' },
  { id: 'replay-mic', category: 'replay', keywords: 'microphone mic voice' },
  { id: 'replay-encoder', category: 'replay', keywords: 'encoder graphics card gpu cpu nvenc performance' },
  { id: 'replay-scale', category: 'replay', keywords: 'resolution 1080p 720p 1440p' },
  { id: 'replay-fps', category: 'replay', keywords: 'fps 30 60 frames' },
  { id: 'replay-quality', category: 'replay', keywords: 'quality bitrate low high' },
  { id: 'replay-folder', category: 'replay', keywords: 'folder clips where saved' },
  { id: 'freeze-hotkey', category: 'freeze', keywords: 'freeze pause f7 hotkey cutscene' },
  { id: 'freeze-saves', category: 'freeze', keywords: 'save folder savegame freeze restore' },
  { id: 'shortcuts', category: 'shortcuts', keywords: 'shortcut key hotkey keybind f8 f9 ctrl' },
  { id: 'screenshot-monitor', category: 'screenshots', keywords: 'screen monitor display screenshot' },
  { id: 'screenshot-folder', category: 'screenshots', keywords: 'folder screenshot pictures' },
  { id: 'extra-folders', category: 'folders', keywords: 'folder games local exe add' },
  { id: 'track-activity', category: 'privacy', keywords: 'privacy playtime streak calendar track' },
  { id: 'active-only', category: 'privacy', keywords: 'playtime active foreground background alt-tab accurate count' },
  { id: 'idle-minutes', category: 'privacy', keywords: 'idle afk inactivity pause playtime' },
  { id: 'streak-threshold', category: 'privacy', keywords: 'streak threshold minutes day' },
  { id: 'clipboard-enabled', category: 'privacy', keywords: 'clipboard history copy' },
  { id: 'clear-activity', category: 'privacy', keywords: 'delete clear history activity' },
  { id: 'data-folders', category: 'data', keywords: 'data folder appdata backup' },
  { id: 'backups', category: 'data', keywords: 'backup restore' },
  { id: 'check-updates', category: 'updates', keywords: 'update version new' },
  { id: 'refresh-artwork', category: 'artwork', keywords: 'cover picture artwork steam fetch' },
  { id: 'artwork-keys', category: 'artwork', keywords: 'igdb steamgriddb key api' },
  { id: 'assistant', category: 'assistant', keywords: 'ai assistant gemini openai groq key' },
];

/** Every setting, titled in the current language. */
export function settingsEntries(): SettingEntry[] {
  return ENTRIES.map((entry) => ({ ...entry, title: t(`si.${entry.id.replace(/-/g, '_')}` as Key) }));
}

export function searchSettings(query: string): SettingEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const words = q.split(/\s+/);
  return settingsEntries().filter((entry) => {
    const haystack = `${entry.title} ${entry.keywords} ${labelOf(entry.category)}`.toLowerCase();
    return words.every((word) => haystack.includes(word));
  });
}

export function labelOf(category: SettingsCategory): string {
  return t(`settings.cat_${category}` as Key);
}
