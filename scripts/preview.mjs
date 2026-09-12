/**
 * Renders the built app in a real browser with a fake Tauri backend, and
 * screenshots each page.
 *
 * The point is to *see* the interface without a Windows machine: every page
 * is driven by the same commands the Rust side answers, so a mock that answers
 * them with plausible data exercises the real components, real CSS and real
 * layout. Screenshots land in `scripts/preview-out/`.
 *
 *   pnpm --filter @gamehub/desktop build && node scripts/preview.mjs
 */
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const dist = join(root, 'apps/desktop/dist');
const L = process.env.PREVIEW_LANG === 'en'
  ? { pick: 'English', library: 'Library', freezes: 'Freeze the game', calendar: 'Calendar', perf: 'Performance', settings: 'Settings', appearance: 'Appearance', search: 'audio', palette: 'freeze', clipboard: 'Clipboard' }
  : { pick: 'Norsk', library: 'Bibliotek', freezes: 'Frys spillet', calendar: 'Kalender', perf: 'Ytelse', settings: 'Innstillinger', appearance: 'Utseende', search: 'lyd', palette: 'frys', clipboard: 'Utklippstavle' };
const out = join(here, 'preview-out');
mkdirSync(out, { recursive: true });

const { chromium } = createRequire(import.meta.url)('playwright');
const CANDIDATES = ['/opt/pw-browsers/chromium-1194/chrome-linux/chrome', '/opt/pw-browsers/chromium/chrome-linux/chrome'];
const executablePath = CANDIDATES.find((p) => existsSync(p));

const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png' };
const server = createServer(async (req, res) => {
  const path = req.url === '/' || req.url.startsWith('/?') ? '/index.html' : req.url.split('?')[0];
  try {
    const body = await readFile(join(dist, path));
    res.writeHead(200, { 'content-type': types[extname(path)] ?? 'application/octet-stream' });
    res.end(body);
  } catch {
    res.writeHead(404);
    res.end();
  }
});
await new Promise((resolve) => server.listen(0, resolve));
const port = server.address().port;

// Real artwork when PREVIEW_ART points at an exported artwork folder: the
// user's own covers, as data URLs. Otherwise a coloured placeholder.
const realArt = (art, kind) => {
  const dir = process.env.PREVIEW_ART;
  if (!dir) return null;
  const candidates = art.endsWith('_custom') ? (kind === 'cover' ? [`${art}.png`] : []) : [`${art}_${kind}.jpg`];
  for (const name of candidates) {
    try {
      const bytes = readFileSync(join(dir, name));
      return `data:image/${name.endsWith('.png') ? 'png' : 'jpeg'};base64,${bytes.toString('base64')}`;
    } catch {}
  }
  return null;
};
// A cover as a data URL, so the grid has pictures.
const cover = (hue) =>
  `data:image/svg+xml;utf8,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="300" height="450"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},70%,45%)"/><stop offset="1" stop-color="hsl(${(hue + 60) % 360},70%,15%)"/></linearGradient></defs><rect width="300" height="450" fill="url(#g)"/><circle cx="150" cy="200" r="70" fill="rgba(255,255,255,.15)"/></svg>`,
  )}`;
const frame = (hue) =>
  `data:image/svg+xml;utf8,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><rect width="640" height="360" fill="hsl(${hue},40%,20%)"/><rect x="40" y="40" width="560" height="280" fill="hsl(${hue},50%,30%)"/></svg>`,
  )}`;

const games = [
  ['steam:1', 'steam', 'Rainbow Six Siege', 200, true, '359550'],
  ['steam:2', 'steam', 'Grand Theft Auto V Enhanced', 45, false, '3240220'],
  ['epic:3', 'epic', 'Fortnite', 120, true, 'epic_Fortnite_custom'],
  ['epic:4', 'epic', 'Rocket League', 30, true, 'epic_Sugar_custom'],
  ['steam:5', 'steam', 'Forza Horizon 5', 280, false, '1551360'],
  ['ea:6', 'ea', 'The Sims 4', 330, false, 'ea__48EBEBBF_B9F8_4520_A3CF_89A730721917__custom'],
  ['steam:7', 'steam', 'House Flipper', 90, false, '613100'],
  ['steam:8', 'steam', 'Supermarket Simulator', 20, false, '2670630'],
  ['steam:9', 'steam', 'Russian Fishing 4', 60, false, '766570'],
  ['steam:10', 'steam', 'Gym Simulator 24', 190, false, '2559270'],
].map(([id, source, name, hue, favorite, art], i) => ({
  id,
  source,
  sourceId: id.split(':')[1],
  name,
  installDir: `C:\\Games\\${name}`,
  installed: true,
  launch: { kind: 'uri', uri: `${source}://run/${i}` },
  sizeBytes: 40e9 + i * 3e9,
  playtimeSeconds: 3600 * (i + 1) * 7,
  lastPlayed: new Date(Date.now() - i * 86_400_000 * 2).toISOString(),
  favorite,
  hidden: false,
  tags: i === 9 ? ['added by hand'] : [],
  metadata: { coverPath: realArt(art, 'cover') ?? cover(hue), heroPath: realArt(art, 'hero'), logoPath: null, description: null, genres: [], releaseDate: null, developer: null, publisher: null, provider: 'steam', fetchedAt: null },
  discoveredAt: '2026-08-30T10:00:00Z',
}));

const settings = {
  startWithWindows: true, minimiseToTray: true, scanIntervalMinutes: 15, autoAddNewGames: true, trackActivity: true, trackActiveOnly: false, activeOnlyChosen: true, idleMinutes: 10,
  streakThresholdMinutes: 15, screenshotFolder: '', screenshotMonitor: 0, clipboardEnabled: true, accent: '', density: 'comfortable',
  backgroundImage: '', replay: { enabled: true, bufferSeconds: 120, fps: 60, quality: 'medium', monitor: 0, systemAudio: true, audioDevice: '', folder: '', scaleHeight: 1080, saveSeconds: 30, encoder: 'auto' },
  extraGameFolders: ['D:\\Spill'], metadata: { igdbClientId: '', igdbClientSecret: '', steamGridDbKey: '' }, ai: { enabled: false, provider: 'gemini', apiKey: '', model: '' },
  cloud: { enabled: false, apiUrl: '' }, language: 'en', languageChosen: false, overlayPopup: true, overlaySound: true, startupAnimation: true, startupSound: true, startupOnReopen: true, theme: 'nattbla', onboarded: true, dismissedUpdateVersion: null,
};

const now = new Date().toISOString();
const freezes = [
  { id: 'f1', gameId: 'steam:1', gameName: 'Rainbow Six Siege', frozenAt: now, state: 'frozen', pids: [1234], folder: 'C:\\Users\\Håkon\\AppData\\Roaming\\GameHub\\Clips\\Rainbow Six Siege\\Frys_2026-09-09', saveDir: 'C:\\Users\\Håkon\\AppData\\Roaming\\R6Siege', saveCopy: '...\\save', saveFiles: 12, saveBytes: 3_400_000, skipped: [], picture: frame(200), note: 'Right before the final round' },
  { id: 'f2', gameId: 'steam:2', gameName: 'Grand Theft Auto V', frozenAt: '2026-09-08T19:12:00Z', state: 'resumed', pids: [999], folder: '...', saveDir: null, saveCopy: null, saveFiles: 0, saveBytes: 0, skipped: [], picture: frame(45), note: '' },
  { id: 'f3', gameId: 'epic:4', gameName: 'Rocket League', frozenAt: '2026-09-05T21:40:00Z', state: 'gone', pids: [], folder: '...', saveDir: 'C:\\...', saveCopy: '...', saveFiles: 40, saveBytes: 22_000_000, skipped: ['C:\\...\\huge.cache'], picture: null, note: '' },
];

const answers = {
  get_library: () => games,
  get_running_games: () => ['steam:1'],
  get_settings: () => settings,
  // The tour itself starts quietly; the opening is captured on its own below.
  startup_greeting: () => ({ animation: false, sound: false }),
  save_settings: (a) => Object.assign(settings, a.settings),
  get_activity: () => ({
    streaks: { current: 6, longest: 14, totalDays: 88, totalSeconds: 412_000, currentStreakGames: ['Rainbow Six Siege', 'Forza Horizon 5', 'Fortnite'], bestMonth: ['2026-07', 98_000] },
    today: { date: now.slice(0, 10), seconds: 5400, games: [['Rainbow Six Siege', 5400]], sessionCount: 2 },
    recent: games.slice(0, 6).map((g, i) => [g.id, g.lastPlayed, 3600 * (i + 1)]),
    current: { gameId: 'steam:1', gameName: 'Rainbow Six Siege', startedAt: now, endedAt: now, seconds: 2520, wallSeconds: 3100 },
    currentActive: true,
    trackingEnabled: true,
  }),
  get_quests: () => ({
    daily: [{ id: 'q1', period: 'daily', title: 'Spill i 30 minutter', description: 'Hva som helst teller.', goal: { kind: 'totalPlaytime', seconds: 1800 }, xp: 200, progress: 1200, target: 1800, complete: false }],
    biweekly: [
      { id: 'q2', period: 'biweekly', title: 'Tilbake til Grand Theft Auto V', description: 'Ikke rørt på en måned.', goal: { kind: 'revisit', gameId: 'steam:2', gameName: 'Grand Theft Auto V', seconds: 1800 }, xp: 500, progress: 1, target: 1, complete: true },
      { id: 'q3', period: 'biweekly', title: 'Tre ulike spill', description: 'Spill tre forskjellige spill.', goal: { kind: 'distinctGames', count: 3 }, xp: 500, progress: 2, target: 3, complete: false },
    ],
    monthly: [{ id: 'q4', period: 'monthly', title: 'Ten hours in Rainbow Six Siege', description: 'Your favourite.', goal: { kind: 'playGame', gameId: 'steam:1', gameName: 'Rainbow Six Siege', seconds: 36000 }, xp: 1000, progress: 21_600, target: 36_000, complete: false }],
    xp: 12_400, level: 2, xpIntoLevel: 2_400, xpForLevel: 10_000,
  }),
  get_calendar_month: (a) => [3, 4, 7, 8, 9].map((d) => ({ date: `${a.month}-${String(d).padStart(2, '0')}`, seconds: 1800 * d, games: [['Rainbow Six Siege', 1200 * d], ['Fortnite', 600 * d]], sessionCount: 2 })),
  replay_status: () => ({ enabled: true, running: true, problem: null, ffmpegPath: 'C:\\...\\ffmpeg.exe', bufferSeconds: 120, bufferedSeconds: 87, minBufferSeconds: 30, maxBufferSeconds: 600, clipFolder: 'C:\\Users\\Håkon\\AppData\\Roaming\\GameHub\\Clips', bufferEstimateMb: 96, audio: { systemAudio: true, systemDevice: 'Høyttalere (Realtek(R) Audio)', microphone: false, note: null } }),
  get_clips: () => [1, 2, 3, 4].map((i) => ({ id: `c${i}`, path: '', gameId: 'steam:1', gameName: i % 2 ? 'Rainbow Six Siege' : 'Fortnite', recordedAt: new Date(Date.now() - i * 3_600_000).toISOString(), seconds: 30 * i, sizeBytes: 42_000_000 * i, favorite: i === 1, hasAudio: i !== 4 })),
  freeze_status: () => ({ supported: true, currentGameId: 'steam:1', currentGameName: 'Rainbow Six Siege', active: freezes[0], saveDir: freezes[0].saveDir, folder: 'C:\\...\\Clips' }),
  get_freezes: () => freezes,
  get_shortcuts: () => [
    ['open_gamehub', 'Open GameHub', 'global', 'Ctrl+Shift+G'], ['quick_tools', 'Quick Tools', 'global', 'Ctrl+Space'], ['screenshot', 'Take a screenshot', 'global', 'F9'],
    ['save_replay', 'Save replay', 'global', 'F8'], ['toggle_replay', 'Replay on/off', 'global', 'Ctrl+Shift+R'], ['freeze_game', 'Freeze', 'global', 'F7'],
    ['toggle_overlay', 'Overlay', 'global', 'Ctrl+Shift+O'], ['search', 'Search', 'app', 'Ctrl+F'], ['library', 'Library', 'app', 'Ctrl+1'], ['home', 'Home', 'app', 'Ctrl+0'],
    ['settings', 'Settings', 'app', 'Ctrl+,'], ['freezes', 'Freezes', 'app', 'Ctrl+Shift+F'], ['rescan', 'Rescan', 'app', 'F5'],
  ].map(([action, label, scope, binding]) => ({ action, label, scope, binding, defaultBinding: binding })),
  get_screenshots: () => [['Rainbow Six Siege', [1, 2, 3].map((i) => ({ id: `s${i}`, path: frame(200 + i * 20), gameId: 'steam:1', gameName: 'Rainbow Six Siege', takenAt: now, sizeBytes: 2e6, favorite: i === 2 }))], ['Desktop', [{ id: 's9', path: frame(10), gameId: null, gameName: 'Desktop', takenAt: now, sizeBytes: 1e6, favorite: false }]]],
  get_clipboard: () => [1, 2, 3].map((i) => ({ id: `k${i}`, text: i === 2 ? 'https://github.com/hakon0607/gamehub' : `Kopiert tekst nummer ${i} — en litt lengre setning for å vise avkorting i listen.`, length: 80, truncated: false, copiedAt: now, pinned: i === 1, isUrl: i === 2 })),
  sample_performance: () => ({ cpuPercent: 37 + Math.random() * 10, cpuName: 'AMD Ryzen 7 7800X3D', cpuCores: 16, memoryUsedBytes: 14e9, memoryTotalBytes: 32e9, swapUsedBytes: 0, diskReadBytes: 0, diskWrittenBytes: 0, networkDownBytes: 3.2e9, networkUpBytes: 0.4e9, disks: [{ name: 'C:', usedBytes: 700e9, totalBytes: 1000e9 }, { name: 'D:', usedBytes: 1.2e12, totalBytes: 2e12 }], gameProcesses: [{ name: 'eldenring.exe', cpuPercent: 24, memoryBytes: 6e9 }], fps: null, gpuPercent: null, cpuTemperatureC: null, unavailableNote: 'FPS, GPU-last og temperaturer krever drivertilgang GameHub ikke har.', sampledAt: now }),
  list_backups: () => [{ id: 'b1', reason: 'update-from-0.6.0', appVersion: '1.0.0', takenAt: now, files: ['library.json', 'settings.json'], bytes: 120_000 }],
  get_recoveries: () => [],
  data_folder: () => ({ data: 'C:\\Users\\Håkon\\AppData\\Roaming\\GameHub', backups: 'C:\\Users\\Håkon\\AppData\\Roaming\\GameHub Backups' }),
  updates_supported: () => true,
  replay_audio_devices: () => ({ devices: ['Mikrofon (Realtek(R) Audio)', 'Stereo Mix (Realtek(R) Audio)'], suggested: 'Stereo Mix (Realtek(R) Audio)' }),
  list_displays: () => ['\\\\.\\DISPLAY1 (2560×1440)', '\\\\.\\DISPLAY2 (1920×1080)'],
  hidden_count: () => 2,
  get_save_folder: () => 'C:\\Users\\Håkon\\AppData\\Roaming\\R6Siege',
  wallpaper_status: () => ({
    supported: true, lockSupported: true,
    desktop: { path: frame(120), fileName: 'nordlys-4k.jpg', width: 3840, height: 2160, bytes: 4_200_000 },
    desktopActive: true,
    lock: null,
    monitors: [
      { id: '\\\\?\\DISPLAY#1', index: 0, width: 2560, height: 1440, current: 'C:\\Users\\Håkon\\AppData\\Roaming\\GameHub\\wallpapers\\desktop-2026.jpg', chosen: null },
      { id: '\\\\?\\DISPLAY#2', index: 1, width: 1920, height: 1080, current: 'C:\\Users\\Håkon\\AppData\\Roaming\\GameHub\\wallpapers\\desktop-2026.jpg', chosen: null },
    ],
    folder: 'C:\\Users\\Håkon\\AppData\\Roaming\\GameHub\\wallpapers',
  }),
  set_wallpaper: () => null,
  'plugin:app|version': () => '1.0.0',
  'plugin:event|listen': (a) => { window.__LISTENERS__ = window.__LISTENERS__ || {}; (window.__LISTENERS__[a.event] ||= []).push(a.handler); return 1; },
  'plugin:event|unlisten': () => null,
  'plugin:updater|check': () => null,
};

const mock = `
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' }, windows: [{label:'main'}], webviews: [{label:'main'}] },
    transformCallback: (cb) => { const id = Math.floor(Math.random() * 1e9); window['_cb' + id] = cb; return id; },
    convertFileSrc: (p) => p,
    invoke: async (cmd, args) => {
      const answers = window.__ANSWERS__;
      if (cmd in answers) return answers[cmd](args ?? {});
      console.warn('unmocked command', cmd);
      return null;
    },
  };
`;

const browser = await chromium.launch(executablePath ? { executablePath } : {});
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
await page.addInitScript(mock);
await page.addInitScript(`window.__ANSWERS__ = {}; window.__ANSWERS_SRC__ = ${JSON.stringify(Object.fromEntries(Object.entries(answers).map(([k, v]) => [k, v.toString()])))};`);
await page.addInitScript(`
  const now = ${JSON.stringify(now)};
  const games = ${JSON.stringify(games)};
  const settings = ${JSON.stringify(settings)};
  const freezes = ${JSON.stringify(freezes)};
  const frame = ${frame.toString()};
  for (const [k, src] of Object.entries(window.__ANSWERS_SRC__)) window.__ANSWERS__[k] = eval('(' + src + ')');
`);
page.on('pageerror', (e) => console.error('page error:', e.message, e.stack?.split('\n').slice(0,3).join(' / ')));
await page.goto(`http://localhost:${port}/`);

const shoot = async (name) => {
  await page.waitForTimeout(700);
  await page.screenshot({ path: join(out, `${name}.png`) });
  console.log(`  ${name}.png`);
};

// First start: the language screen, English preselected. Norwegian is
// chosen here so the rest of the tour reads as before.
await page.waitForSelector('.onboarding');
await shoot('00-language');
await page.click(`.theme-card:has-text("${L.pick}")`);
await page.click('.btn-accent.big');
await page.waitForSelector('.sidebar');
await page.waitForTimeout(900);

await shoot('01-home');
const nav = async (label) => {
  await page.click(`.nav-item:has-text("${label}")`);
};
await nav(L.library);
await shoot('02-library');
await page.click('.card-open >> nth=0');
await shoot('03-game');
await nav('Replay');
await shoot('04-replay');
await nav(L.freezes);
await shoot('05-freezes');
await nav('Screenshots');
await shoot('06-screenshots');
await nav('Quests');
await shoot('07-quests');
await nav(L.calendar);
await shoot('08-calendar');
await nav(L.perf);
await shoot('09-performance');
await nav(L.settings);
await shoot('10-settings-general');
await page.click(`.settings-tab:has-text("${L.appearance}")`);
await page.evaluate(() => document.querySelector('.wp')?.scrollIntoView());
await shoot('10b-settings-wallpapers');
await page.click('.settings-tab:has-text("Replay")');
await shoot('11-settings-replay');
await page.fill('.settings-search input', L.search);
await shoot('12-settings-search');
await page.keyboard.press('Escape');
await page.keyboard.press('Control+k');
await page.waitForSelector('.palette-input');
await page.fill('.palette-input', L.palette);
await shoot('13-palette');
await page.keyboard.press('Escape');
await nav('Streaks');
await shoot('14-streaks');
await nav(L.clipboard);
await shoot('15-clipboard');

// The same app in three other languages — the words all change, nothing else.
for (const [code, home, settingsLabel] of [['de', 'Start', 'Einstellungen'], ['es', 'Inicio', 'Ajustes'], ['pl', 'Start', 'Ustawienia']]) {
  await page.addInitScript(`window.__ANSWERS__.get_settings = () => ({ ...${JSON.stringify(settings)}, language: ${JSON.stringify(code)}, languageChosen: true });`);
  await page.reload();
  await page.waitForSelector('.sidebar');
  await page.waitForTimeout(900);
  await shoot(`20-home-${code}`);
  await page.click(`.nav-item:has-text("${settingsLabel}")`);
  await shoot(`21-settings-${code}`);
  void home;
}
// The popup over the game, as the hidden app shows it: its own tiny window.
const popup = await browser.newPage({ viewport: { width: 400, height: 104 }, deviceScaleFactor: 2 });
await popup.addInitScript(mock);
await popup.addInitScript(`window.__ANSWERS__ = {}; window.__ANSWERS_SRC__ = ${JSON.stringify(Object.fromEntries(Object.entries(answers).map(([k, v]) => [k, v.toString()])))};`);
await popup.addInitScript(`for (const [k, src] of Object.entries(window.__ANSWERS_SRC__)) window.__ANSWERS__[k] = eval('(' + src + ')');`);
await popup.goto(`http://localhost:${port}/?overlay=1`);
await popup.waitForSelector('.overlay-stack');
// Stand in for the game underneath the transparent window.
await popup.addStyleTag({ content: 'html.overlay-window { background: linear-gradient(135deg, #1a2a1a, #0b1a2b) !important; }' });
await popup.evaluate(() => {
  for (const id of window.__LISTENERS__['overlay-toast'] ?? []) {
    window['_cb' + id]({ payload: { title: '@toast_clip_saved', body: 'Fortnite · 30 s', kind: 'clip', language: 'nb', theme: 'nattbla', seconds: 60 } });
  }
});
await popup.waitForTimeout(500);
await popup.screenshot({ path: join(out, '30-popup.png') });
console.log('  30-popup.png');
// The opening: the logo animation, frame by frame, on a fresh page that
// answers "yes" to the greeting. Time is driven by hand, because a full
// screenshot takes longer than the animation gives it.
const opening = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
await opening.addInitScript(mock);
await opening.addInitScript(`window.__ANSWERS__ = {}; window.__ANSWERS_SRC__ = ${JSON.stringify(Object.fromEntries(Object.entries(answers).map(([k, v]) => [k, v.toString()])))};`);
await opening.addInitScript(`
  const now = ${JSON.stringify(now)};
  const games = ${JSON.stringify(games)};
  const settings = ${JSON.stringify({ ...settings, languageChosen: true })};
  const freezes = ${JSON.stringify(freezes)};
  const frame = ${frame.toString()};
  for (const [k, src] of Object.entries(window.__ANSWERS_SRC__)) window.__ANSWERS__[k] = eval('(' + src + ')');
  window.__ANSWERS__.get_settings = () => settings;
  window.__ANSWERS__.startup_greeting = () => ({ animation: true, sound: true });
`);
await opening.clock.install();
await opening.goto(`http://localhost:${port}/`);
await opening.waitForSelector('.splash-playing');
await opening.clock.pauseAt(await opening.evaluate(() => Date.now()));
let at = 0;
for (const [i, ms] of [150, 450, 800, 1200, 1500, 1750].entries()) {
  await opening.clock.runFor(ms - at);
  at = ms;
  await opening.waitForTimeout(80);
  await opening.screenshot({ path: join(out, `40-opening-${i}.png`) });
  console.log(`  40-opening-${i}.png  (${ms} ms)`);
}
await opening.clock.runFor(1000);
await opening.waitForSelector('.splash', { state: 'detached' });
await opening.screenshot({ path: join(out, '40-opening-done.png') });
console.log('  40-opening-done.png');

await browser.close();
server.close();
console.log(`\nscreenshots in ${out}`);
