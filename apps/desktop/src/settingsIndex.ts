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
  title: string;
  /** Words people might type, in Norwegian and English. */
  keywords: string;
}

export const CATEGORIES: { id: SettingsCategory; label: string; icon: string; blurb: string }[] = [
  { id: 'general', label: 'Generelt', icon: '⚙', blurb: 'Oppstart, systemkurv og hvor ofte GameHub ser etter nye spill.' },
  { id: 'appearance', label: 'Utseende', icon: '◐', blurb: 'Tema, farge, tetthet, animasjoner og bakgrunnsbilde.' },
  { id: 'replay', label: 'Replay', icon: '⏺', blurb: 'Bufferlengde, lyd, kvalitet og hva F8 lagrer.' },
  { id: 'freeze', label: 'Frys spillet', icon: '❄', blurb: 'Hurtigtast, lagringsmapper og hvor frysepunktene havner.' },
  { id: 'shortcuts', label: 'Hurtigtaster', icon: '⌨', blurb: 'Alle tastene, og hva de gjør. Alt kan bindes om.' },
  { id: 'screenshots', label: 'Screenshots', icon: '⎙', blurb: 'Hvilken skjerm som fanges, og hvor bildene havner.' },
  { id: 'folders', label: 'Spillmapper', icon: '▤', blurb: 'Ekstra mapper GameHub skal lete i etter spill.' },
  { id: 'privacy', label: 'Personvern', icon: '◈', blurb: 'Hva som spores om spillingen din, og hvordan du sletter det.' },
  { id: 'data', label: 'Dine data', icon: '⛁', blurb: 'Hvor alt ligger, sikkerhetskopier, og hvordan du henter dem tilbake.' },
  { id: 'updates', label: 'Oppdateringer', icon: '↑', blurb: 'Versjonen du kjører, og automatiske oppdateringer.' },
  { id: 'artwork', label: 'Coverbilder', icon: '▣', blurb: 'Hent manglende covere, og egne nøkler hvis du har dem.' },
  { id: 'assistant', label: 'Assistent', icon: '✦', blurb: 'AI-hjelp i appen. Av som standard, og krever din egen nøkkel.' },
];

export const SETTINGS: SettingEntry[] = [
  { id: 'start-with-windows', category: 'general', title: 'Start med Windows', keywords: 'oppstart autostart boot start windows' },
  { id: 'minimise-to-tray', category: 'general', title: 'Fortsett i systemkurven når vinduet lukkes', keywords: 'tray systemkurv lukk minimer bakgrunn' },
  { id: 'scan-interval', category: 'general', title: 'Se etter nye spill', keywords: 'skann scan intervall nye spill automatisk' },
  { id: 'auto-add', category: 'general', title: 'Legg til nye spill automatisk', keywords: 'auto add nye spill oppdag' },
  { id: 'language', category: 'general', title: 'Språk', keywords: 'språk language norsk engelsk' },

  { id: 'theme', category: 'appearance', title: 'Tema', keywords: 'tema theme farge mørk blå svart utseende' },
  { id: 'accent', category: 'appearance', title: 'Aksentfarge', keywords: 'accent farge color' },
  { id: 'density', category: 'appearance', title: 'Tetthet', keywords: 'kompakt tett spacing density' },
  { id: 'background', category: 'appearance', title: 'Bakgrunnsbilde', keywords: 'bakgrunn bilde wallpaper background' },

  { id: 'replay-enabled', category: 'replay', title: 'Replay på/av', keywords: 'replay opptak record buffer på av' },
  { id: 'replay-buffer', category: 'replay', title: 'Hvor mye som huskes (bufferlengde)', keywords: 'buffer lengde minutter sekunder husk opptak 10 min' },
  { id: 'replay-save', category: 'replay', title: 'Hvor mye F8 lagrer', keywords: 'f8 lagre klipp lengde sekunder' },
  { id: 'replay-system-audio', category: 'replay', title: 'Ta opp spillyd (det du hører)', keywords: 'lyd audio sound system høyttaler spillyd loopback' },
  { id: 'replay-mic', category: 'replay', title: 'Mikrofon', keywords: 'mikrofon mic lyd stemme' },
  { id: 'replay-encoder', category: 'replay', title: 'Hvem koder videoen', keywords: 'encoder skjermkort gpu cpu nvenc ytelse' },
  { id: 'replay-scale', category: 'replay', title: 'Oppløsning på opptaket', keywords: 'oppløsning 1080p 720p 1440p resolution' },
  { id: 'replay-fps', category: 'replay', title: 'Bilder per sekund', keywords: 'fps 30 60 bilder' },
  { id: 'replay-quality', category: 'replay', title: 'Kvalitet', keywords: 'kvalitet bitrate quality lav høy' },
  { id: 'replay-folder', category: 'replay', title: 'Mappe for klipp og frysepunkter', keywords: 'mappe folder klipp lagres hvor' },

  { id: 'freeze-hotkey', category: 'freeze', title: 'Hurtigtast for frys', keywords: 'frys freeze pause f7 hurtigtast cutscene' },
  { id: 'freeze-saves', category: 'freeze', title: 'Lagringsmapper per spill', keywords: 'save lagring mappe savegame frys gjenopprett' },

  { id: 'shortcuts', category: 'shortcuts', title: 'Alle hurtigtaster', keywords: 'hurtigtast tast shortcut hotkey keybind f8 f9 ctrl' },

  { id: 'screenshot-monitor', category: 'screenshots', title: 'Skjerm som fanges', keywords: 'skjerm monitor display screenshot' },
  { id: 'screenshot-folder', category: 'screenshots', title: 'Mappe for screenshots', keywords: 'mappe folder screenshot bilder' },

  { id: 'extra-folders', category: 'folders', title: 'Ekstra spillmapper', keywords: 'mappe folder spill lokale exe legg til' },

  { id: 'track-activity', category: 'privacy', title: 'Registrer spilletid, streaks og kalender', keywords: 'personvern privacy spilletid streak kalender spor' },
  { id: 'streak-threshold', category: 'privacy', title: 'En dag teller etter', keywords: 'streak terskel minutter dag' },
  { id: 'clipboard-enabled', category: 'privacy', title: 'Utklippshistorikk', keywords: 'utklipp clipboard historikk kopier' },
  { id: 'clear-activity', category: 'privacy', title: 'Slett all aktivitet', keywords: 'slett tøm historikk aktivitet' },

  { id: 'data-folders', category: 'data', title: 'Hvor dataene ligger', keywords: 'data mappe appdata backup sikkerhetskopi' },
  { id: 'backups', category: 'data', title: 'Sikkerhetskopier', keywords: 'backup sikkerhetskopi gjenopprett restore' },

  { id: 'check-updates', category: 'updates', title: 'Se etter oppdateringer', keywords: 'oppdatering update versjon ny' },
  { id: 'refresh-artwork', category: 'artwork', title: 'Hent manglende coverbilder', keywords: 'cover bilde artwork steam hent' },
  { id: 'artwork-keys', category: 'artwork', title: 'IGDB og SteamGridDB-nøkler', keywords: 'igdb steamgriddb nøkkel key api' },
  { id: 'assistant', category: 'assistant', title: 'Assistent', keywords: 'ai assistent gemini openai groq nøkkel' },
];

export function searchSettings(query: string): SettingEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  const words = q.split(/\s+/);
  return SETTINGS.filter((entry) => {
    const haystack = `${entry.title} ${entry.keywords} ${labelOf(entry.category)}`.toLowerCase();
    return words.every((word) => haystack.includes(word));
  });
}

export function labelOf(category: SettingsCategory): string {
  return CATEGORIES.find((c) => c.id === category)?.label ?? category;
}
