import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import type { Update } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type AudioOptions, type Settings } from '../api';
import { checkForUpdate, describeUpdateError, type UpdateStatus } from '../updater';
import { ShortcutCenter } from '../components/ShortcutCenter';
import { DataSection } from '../components/DataSection';
import { CATEGORIES, searchSettings, type SettingsCategory } from '../settingsIndex';
import { Confirm, Kbd, SettingGroup, SettingRow, Slider, Toggle } from '../ui';
import { formatSeconds } from '../format';

const THEMES: { id: string; name: string; swatch: string[] }[] = [
  { id: 'nattbla', name: 'Nattblå', swatch: ['#05070d', '#0b1120', '#4f8cff'] },
  { id: 'midnight', name: 'Midnatt', swatch: ['#020308', '#070b16', '#6d8bff'] },
  { id: 'ocean', name: 'Hav', swatch: ['#04101a', '#0a1c2b', '#29b6e8'] },
  { id: 'purple-space', name: 'Lilla rom', swatch: ['#07051a', '#120e2e', '#a583ff'] },
  { id: 'neon', name: 'Neon', swatch: ['#050b09', '#0b1712', '#2ef2a0'] },
  { id: 'minimal', name: 'Minimal', swatch: ['#0e0e10', '#17171a', '#d8d8e0'] },
  { id: 'daylight', name: 'Dagslys', swatch: ['#eef1f7', '#ffffff', '#2456d6'] },
];

/**
 * Settings: one category at a time on the right, the list on the left, and a
 * search box that finds a setting by what it does rather than where it is.
 * Landing on a setting from search (or from the command palette) highlights
 * the row so it is obvious which one was meant.
 */
export function SettingsView({
  settings,
  onSaved,
  onToast,
  onUpdateFound,
  initialCategory,
  focusSetting,
}: {
  settings: Settings | null;
  onSaved: (settings: Settings) => void;
  onToast: (title: string, body?: string) => void;
  onUpdateFound: (found: { status: UpdateStatus; update: Update }) => void;
  initialCategory?: SettingsCategory;
  focusSetting?: string | null;
}) {
  const [draft, setDraft] = useState<Settings | null>(settings);
  const [tab, setTab] = useState<SettingsCategory>(initialCategory ?? 'general');
  const [query, setQuery] = useState('');
  const [highlight, setHighlight] = useState<string | null>(focusSetting ?? null);
  const [checking, setChecking] = useState(false);
  const [canUpdate, setCanUpdate] = useState(false);
  const [version, setVersion] = useState('');
  const [displays, setDisplays] = useState<string[]>([]);
  const [audio, setAudio] = useState<AudioOptions>({ devices: [], suggested: null });
  const [clearing, setClearing] = useState(false);

  useEffect(() => setDraft(settings), [settings]);
  useEffect(() => {
    void api.updatesSupported().then(setCanUpdate);
    void getVersion().then(setVersion);
    void api.listDisplays().then(setDisplays);
    void api.replayAudioDevices().then(setAudio);
  }, []);
  useEffect(() => {
    if (initialCategory) setTab(initialCategory);
    setHighlight(focusSetting ?? null);
  }, [initialCategory, focusSetting]);

  // Scroll the highlighted row into view once it exists, then let the glow fade.
  useEffect(() => {
    if (!highlight) return;
    const el = document.querySelector(`[data-setting="${highlight}"]`);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    const timer = window.setTimeout(() => setHighlight(null), 2600);
    return () => window.clearTimeout(timer);
  }, [highlight, tab]);

  const hits = useMemo(() => searchSettings(query), [query]);

  if (!draft) return <p className="empty">Laster …</p>;

  const save = async (next: Settings) => {
    setDraft(next);
    try {
      const saved = await api.saveSettings(next);
      onSaved(saved);
      setDraft(saved);
    } catch (error) {
      onToast('Innstillingen ble ikke lagret', String(error));
    }
  };
  const replay = (patch: Partial<Settings['replay']>) => save({ ...draft, replay: { ...draft.replay, ...patch } });
  const active = CATEGORIES.find((c) => c.id === tab) ?? CATEGORIES[0]!;
  const row = (id: string, title: string, description: ReactNode, control: ReactNode, index = 0) => (
    <SettingRow id={id} title={title} description={description} highlight={highlight === id} index={index}>
      {control}
    </SettingRow>
  );

  return (
    <div className="settings view">
      <nav className="settings-nav" aria-label="Innstillinger">
        <div className="search-wrap settings-search" style={{ maxWidth: 'none' }}>
          <span className="search-icon">⌕</span>
          <input
            className="search"
            placeholder="Finn en innstilling …"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && hits[0]) {
                setTab(hits[0].category);
                setHighlight(hits[0].id);
                setQuery('');
              }
              if (e.key === 'Escape') setQuery('');
            }}
          />
        </div>
        {query ? (
          hits.length === 0 ? (
            <p className="palette-empty">Ingen innstilling passer «{query}»</p>
          ) : (
            hits.map((hit) => (
              <button
                key={hit.id}
                className="settings-tab"
                onClick={() => {
                  setTab(hit.category);
                  setHighlight(hit.id);
                  setQuery('');
                }}
              >
                <span className="settings-tab-icon" aria-hidden="true">
                  {CATEGORIES.find((c) => c.id === hit.category)?.icon}
                </span>
                <span>
                  {hit.title}
                  <small>{CATEGORIES.find((c) => c.id === hit.category)?.label}</small>
                </span>
              </button>
            ))
          )
        ) : (
          CATEGORIES.map((category) => (
            <button key={category.id} className="settings-tab" aria-current={tab === category.id} onClick={() => setTab(category.id)}>
              <span className="settings-tab-icon" aria-hidden="true">
                {category.icon}
              </span>
              <span>{category.label}</span>
            </button>
          ))
        )}
      </nav>

      <section className="settings-panel" key={tab} aria-live="polite">
        <div className="page-head" style={{ marginBottom: 16 }}>
          <div>
            <h1>{active.label}</h1>
            <p>{active.blurb}</p>
          </div>
        </div>

        {tab === 'general' && (
          <>
            <SettingGroup title="Oppstart">
              {row('start-with-windows', 'Start med Windows', 'GameHub starter i systemkurven når du logger inn, så nye spill oppdages med en gang.', <Toggle checked={draft.startWithWindows} onChange={(v) => void save({ ...draft, startWithWindows: v })} />)}
              {row('minimise-to-tray', 'Fortsett i systemkurven når vinduet lukkes', 'Avslutt fra menyen på ikonet nede til høyre. I kurven bruker GameHub ingen målbar prosessorkraft.', <Toggle checked={draft.minimiseToTray} onChange={(v) => void save({ ...draft, minimiseToTray: v })} />, 1)}
            </SettingGroup>
            <SettingGroup title="Skanning">
              {row('scan-interval', 'Se etter nye spill', 'Mappene launcherne skriver til overvåkes uansett; dette er et sikkerhetsnett.', (
                <select value={draft.scanIntervalMinutes} onChange={(e) => void save({ ...draft, scanIntervalMinutes: Number(e.target.value) })}>
                  <option value={0}>Bare når noe endrer seg</option>
                  <option value={15}>Hvert 15. minutt</option>
                  <option value={60}>Hver time</option>
                  <option value={360}>Hver 6. time</option>
                </select>
              ))}
              {row('auto-add', 'Legg til nye spill automatisk', 'Et spill som dukker opp i en launcher havner i biblioteket uten at du gjør noe.', <Toggle checked={draft.autoAddNewGames} onChange={(v) => void save({ ...draft, autoAddNewGames: v })} />, 1)}
            </SettingGroup>
          </>
        )}

        {tab === 'appearance' && (
          <>
            <SettingGroup title="Tema">
              <div style={{ padding: 16 }} data-setting="theme" className={highlight === 'theme' ? 'highlight' : ''}>
                <div className="themes">
                  {THEMES.map((theme) => (
                    <button key={theme.id} className="theme-card" aria-pressed={(draft.theme || 'nattbla') === theme.id} onClick={() => void save({ ...draft, theme: theme.id })}>
                      <span className="theme-swatch">
                        {theme.swatch.map((colour) => <i key={colour} style={{ background: colour }} />)}
                      </span>
                      <p>{theme.name}</p>
                    </button>
                  ))}
                </div>
              </div>
            </SettingGroup>
            <SettingGroup title="Detaljer">
              {row('accent', 'Aksentfarge', 'Fargen på knapper, glød og markeringer. Tom betyr temaets egen.', (
                <>
                  <input type="color" value={draft.accent || '#4f8cff'} onChange={(e) => void save({ ...draft, accent: e.target.value })} />
                  {draft.accent && <button className="btn sm btn-ghost" onClick={() => void save({ ...draft, accent: '' })}>Temaets egen</button>}
                </>
              ))}
              {row('density', 'Tetthet', 'Kompakt får plass til flere spill per rad.', (
                <select value={draft.density} onChange={(e) => void save({ ...draft, density: e.target.value })}>
                  <option value="comfortable">Behagelig</option>
                  <option value="compact">Kompakt</option>
                </select>
              ), 1)}
              {row('background', 'Bakgrunnsbilde', draft.backgroundImage ? <code>{draft.backgroundImage}</code> : 'Ligger bak den animerte bakgrunnen. Blir på denne PC-en.', (
                <>
                  <button className="btn sm" onClick={async () => {
                    const picked = await open({ multiple: false, filters: [{ name: 'Bilde', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
                    if (typeof picked === 'string') void save({ ...draft, backgroundImage: picked });
                  }}>Velg…</button>
                  {draft.backgroundImage && <button className="btn sm btn-ghost" onClick={() => void save({ ...draft, backgroundImage: '' })}>Fjern</button>}
                </>
              ), 2)}
            </SettingGroup>
          </>
        )}

        {tab === 'replay' && (
          <>
            <SettingGroup title="Opptak">
              {row('replay-enabled', 'Replay', <>Tar opp skjermen fortløpende. Kan også slås av og på i spill med <Kbd>Ctrl+Shift+R</Kbd>.</>, (
                <Toggle checked={draft.replay.enabled} onChange={async (v) => {
                  try { await api.setReplayEnabled(v); } catch (error) { onToast('Replay startet ikke', String(error)); return; }
                  onSaved(await api.getSettings());
                }} />
              ))}
              {row('replay-buffer', 'Hvor mye som huskes', `Alt eldre enn dette overskrives fortløpende og havner aldri på disken. 30 sek – 10 min.`, (
                <Slider value={draft.replay.bufferSeconds} min={30} max={600} step={15} format={formatSeconds}
                  onCommit={(s) => s !== draft.replay.bufferSeconds && void replay({ bufferSeconds: s, saveSeconds: Math.min(draft.replay.saveSeconds, s) })} />
              ), 1)}
              {row('replay-save', 'Hvor mye F8 lagrer', 'Kortere opptak enn dette gir et kortere klipp, aldri en feil.', (
                <select value={draft.replay.saveSeconds} onChange={(e) => void replay({ saveSeconds: Number(e.target.value) })}>
                  {[15, 30, 60, 120, 180, 300, 600].filter((s) => s <= draft.replay.bufferSeconds).map((s) => <option key={s} value={s}>{formatSeconds(s)}</option>)}
                </select>
              ), 2)}
              {row('replay-folder', 'Mappe for klipp og frysepunkter', draft.replay.folder ? <code>{draft.replay.folder}</code> : 'Standard: «Clips» i datamappen. Hvert spill får sin egen undermappe.', (
                <>
                  <button className="btn sm" onClick={async () => {
                    const picked = await open({ directory: true, multiple: false });
                    if (typeof picked === 'string') void replay({ folder: picked });
                  }}>Velg…</button>
                  {draft.replay.folder && <button className="btn sm btn-ghost" onClick={() => void replay({ folder: '' })}>Standard</button>}
                </>
              ), 3)}
            </SettingGroup>
            <SettingGroup title="Lyd">
              {row('replay-system-audio', 'Ta opp spillyd', 'Det du hører i høyttalerne eller hodetelefonene, uten «Stereo Mix» eller andre triks. Følger standard lydenhet i Windows.', <Toggle checked={draft.replay.systemAudio} onChange={(v) => void replay({ systemAudio: v })} />)}
              {row('replay-mic', 'Mikrofon', audio.devices.length === 0 ? 'Ingen mikrofon funnet. Blandes inn med spillyden når en er valgt.' : 'Blandes inn med spillyden.', (
                <select value={draft.replay.audioDevice || 'none'} onChange={(e) => void replay({ audioDevice: e.target.value === 'none' ? '' : e.target.value })}>
                  <option value="none">Ingen</option>
                  {audio.devices.map((d) => <option key={d} value={d}>{d}{d === audio.suggested ? ' (loopback)' : ''}</option>)}
                  {draft.replay.audioDevice && !audio.devices.includes(draft.replay.audioDevice) && <option value={draft.replay.audioDevice}>{draft.replay.audioDevice} — ikke funnet nå</option>}
                </select>
              ), 1)}
            </SettingGroup>
            <SettingGroup title="Bilde og ytelse">
              {row('replay-encoder', 'Hvem koder videoen', 'Skjermkortet har en egen videomotor som står ubrukt mens du spiller. Faller tilbake til prosessoren hvis kortet ikke kan.', (
                <select value={draft.replay.encoder} onChange={(e) => void replay({ encoder: e.target.value })}>
                  <option value="auto">Skjermkortet hvis mulig (anbefalt)</option>
                  <option value="cpu">Prosessoren</option>
                </select>
              ))}
              {row('replay-scale', 'Oppløsning på opptaket', 'Aldri høyere enn skjermen din. Det enkleste stedet å hente inn ytelse.', (
                <select value={draft.replay.scaleHeight} onChange={(e) => void replay({ scaleHeight: Number(e.target.value) })}>
                  <option value={720}>720p — lettest</option>
                  <option value={1080}>1080p — anbefalt</option>
                  <option value={1440}>1440p</option>
                  <option value={0}>Samme som skjermen — tyngst</option>
                </select>
              ), 1)}
              {row('replay-fps', 'Bilder per sekund', '', (
                <select value={draft.replay.fps} onChange={(e) => void replay({ fps: Number(e.target.value) })}>
                  <option value={30}>30</option>
                  <option value={60}>60</option>
                </select>
              ), 2)}
              {row('replay-quality', 'Kvalitet', 'Høyere kvalitet betyr mer disk per minutt i bufferet.', (
                <select value={draft.replay.quality} onChange={(e) => void replay({ quality: e.target.value })}>
                  <option value="low">Lav — 3 Mbit/s</option>
                  <option value="medium">Middels — 6 Mbit/s</option>
                  <option value="high">Høy — 12 Mbit/s</option>
                </select>
              ), 3)}
            </SettingGroup>
            <p className="note">Et spill i eksklusiv fullskjerm kan ikke fanges; bytt til «rammeløst vindu» i spillet. Det tilbyr nesten alle moderne spill.</p>
          </>
        )}

        {tab === 'freeze' && (
          <>
            <SettingGroup title="Hurtigtast">
              <div data-setting="freeze-hotkey" className={highlight === 'freeze-hotkey' ? 'highlight' : ''}>
                <ShortcutCenter onToast={onToast} filter={['freeze_game', 'freezes']} />
              </div>
            </SettingGroup>
            <SettingGroup title="Lagringsmapper">
              {row('freeze-saves', 'Lagringsmappe per spill', 'Velges på hvert spills side («Finn» leter i de vanlige mappene). Med en lagringsmappe tar en frys også kopi av lagringen, som overlever en omstart.', <span className="badge-pill">På spillsiden</span>)}
              {row('freeze-folder', 'Hvor frysepunktene havner', <>Samme mappe som replay-klippene, under spillets undermappe: <code>{draft.replay.folder || 'Clips'}\Spillnavn\Frys_…</code></>, <button className="btn sm" onClick={() => setTab('replay')}>Endre</button>, 1)}
            </SettingGroup>
            <p className="note">
              Frysing stopper alle spillets prosesser med Windows sin egen mekanisme (NtSuspendProcess). Spill med anti-juks
              (EasyAntiCheat, BattlEye, Vanguard) beskytter prosessene sine og kan ikke fryses — GameHub sier fra hvis det skjer.
            </p>
          </>
        )}

        {tab === 'shortcuts' && (
          <div data-setting="shortcuts">
            <p className="note" style={{ marginBottom: 14 }}>Klikk på en tast for å endre den, og trykk den nye kombinasjonen. To handlinger kan ikke dele samme kombinasjon — GameHub sier fra hvilken som har den.</p>
            <ShortcutCenter onToast={onToast} />
          </div>
        )}

        {tab === 'screenshots' && (
          <SettingGroup>
            {row('screenshot-monitor', 'Skjerm som fanges', 'Gjelder både F9 og bildet som tas ved en frys.', (
              <select value={draft.screenshotMonitor} onChange={(e) => void save({ ...draft, screenshotMonitor: Number(e.target.value) })}>
                {displays.length === 0 ? <option value={0}>Hovedskjermen</option> : displays.map((name, index) => <option key={name} value={index}>{name}</option>)}
              </select>
            ))}
            {row('screenshot-folder', 'Mappe for screenshots', draft.screenshotFolder ? <code>{draft.screenshotFolder}</code> : 'Standard: «Screenshots» i datamappen.', (
              <>
                <button className="btn sm" onClick={async () => {
                  const picked = await open({ directory: true, multiple: false });
                  if (typeof picked === 'string') void save({ ...draft, screenshotFolder: picked });
                }}>Velg…</button>
                {draft.screenshotFolder && <button className="btn sm btn-ghost" onClick={() => void save({ ...draft, screenshotFolder: '' })}>Standard</button>}
                <button className="btn sm btn-ghost" onClick={async () => void revealItemInDir(await api.screenshotFolder())}>Åpne</button>
              </>
            ), 1)}
          </SettingGroup>
        )}

        {tab === 'folders' && (
          <SettingGroup title="Ekstra spillmapper">
            <div data-setting="extra-folders" className={highlight === 'extra-folders' ? 'highlight' : ''}>
              <p className="note" style={{ padding: '12px 16px 4px' }}>Spill som ikke hører til noen launcher. Hver undermappe behandles som ett spill; .exe-filen velges automatisk.</p>
              {draft.extraGameFolders.map((folder, index) => (
                <SettingRow key={folder} title={folder} index={index}>
                  <button className="btn sm btn-ghost btn-danger" onClick={() => void save({ ...draft, extraGameFolders: draft.extraGameFolders.filter((f) => f !== folder) })}>Fjern</button>
                </SettingRow>
              ))}
              <div style={{ padding: 16 }}>
                <button className="btn" onClick={async () => {
                  const picked = await open({ directory: true, multiple: false });
                  if (typeof picked !== 'string') return;
                  const saved = await api.addGameFolder(picked);
                  onSaved(saved);
                  setDraft(saved);
                }}>+ Legg til mappe</button>
              </div>
            </div>
          </SettingGroup>
        )}

        {tab === 'privacy' && (
          <>
            <SettingGroup title="Aktivitet">
              {row('track-activity', 'Registrer spilletid, streaks og kalender', 'Alt blir på denne PC-en. Av stopper ny registrering med en gang; å starte spill påvirkes ikke.', <Toggle checked={draft.trackActivity} onChange={(v) => void save({ ...draft, trackActivity: v })} />)}
              {row('streak-threshold', 'En dag teller mot streaken etter', '', (
                <select value={draft.streakThresholdMinutes} onChange={(e) => void save({ ...draft, streakThresholdMinutes: Number(e.target.value) })}>
                  {[5, 15, 30, 60].map((m) => <option key={m} value={m}>{m} minutter</option>)}
                </select>
              ), 1)}
              {row('clipboard-enabled', 'Utklippshistorikk', 'De siste 100 tingene du kopierer. Av stopper også overvåkingen helt.', <Toggle checked={draft.clipboardEnabled} onChange={(v) => void save({ ...draft, clipboardEnabled: v })} />, 2)}
            </SettingGroup>
            <SettingGroup title="Slett">
              {row('clear-activity', 'Slett all aktivitet', 'Spilletid, streaks og kalender. Spill og innstillinger beholdes.', <button className="btn sm btn-danger" onClick={() => setClearing(true)}>Slett historikken</button>)}
            </SettingGroup>
          </>
        )}

        {tab === 'data' && <DataSection onToast={onToast} />}

        {tab === 'updates' && (
          <SettingGroup title={`GameHub ${version}`}>
            {row('check-updates', 'Automatiske oppdateringer', canUpdate
              ? 'GameHub sjekker GitHub kort etter oppstart og spør før noe installeres. Oppdateringen er signert og sjekkes før den kjøres.'
              : 'Denne utgaven er bygget uten oppdateringsnøkkel (for eksempel lokalt med BUILD.bat) og oppdaterer seg ikke selv. Last ned nyeste fra GitHub Releases.', (
              <button className={`btn sm${checking ? ' busy' : ''}`} disabled={checking || !canUpdate} onClick={async () => {
                setChecking(true);
                try {
                  const found = await checkForUpdate();
                  if (found.update && found.status.available) onUpdateFound({ status: found.status, update: found.update });
                  else onToast('Du har nyeste versjon.', `GameHub ${found.status.currentVersion}`);
                } catch (error) {
                  onToast('Kunne ikke se etter oppdateringer', describeUpdateError(error));
                } finally {
                  setChecking(false);
                }
              }}>Se etter oppdateringer</button>
            ))}
          </SettingGroup>
        )}

        {tab === 'artwork' && (
          <>
            <SettingGroup title="Covere">
              {row('refresh-artwork', 'Hent manglende coverbilder', 'Steam-covere hentes automatisk og trenger ingen nøkkel. Spill fra andre launchere matches på navn.', (
                <button className="btn sm" onClick={async () => { await api.refreshArtwork(); onToast('Leter etter coverbilder som mangler …'); }}>Hent nå</button>
              ))}
            </SettingGroup>
            <SettingGroup title="Egne nøkler (valgfritt, ikke koblet på ennå)">
              <div data-setting="artwork-keys" className={highlight === 'artwork-keys' ? 'highlight' : ''}>
                {row('igdb-id', 'IGDB client id', '', <input type="text" value={draft.metadata.igdbClientId} onChange={(e) => setDraft({ ...draft, metadata: { ...draft.metadata, igdbClientId: e.target.value } })} onBlur={() => void save(draft)} />)}
                {row('igdb-secret', 'IGDB client secret', '', <input type="password" value={draft.metadata.igdbClientSecret} onChange={(e) => setDraft({ ...draft, metadata: { ...draft.metadata, igdbClientSecret: e.target.value } })} onBlur={() => void save(draft)} />, 1)}
                {row('sgdb', 'SteamGridDB-nøkkel', '', <input type="password" value={draft.metadata.steamGridDbKey} onChange={(e) => setDraft({ ...draft, metadata: { ...draft.metadata, steamGridDbKey: e.target.value } })} onBlur={() => void save(draft)} />, 2)}
              </div>
            </SettingGroup>
          </>
        )}

        {tab === 'assistant' && (
          <SettingGroup>
            <div data-setting="assistant" className={highlight === 'assistant' ? 'highlight' : ''}>
              {row('ai-enabled', 'Assistent', 'Kan svare på «hva skal jeg spille?» ut fra biblioteket ditt. Navn og timer sendes til leverandøren du velger — aldri filer.', <Toggle checked={draft.ai.enabled} onChange={(v) => void save({ ...draft, ai: { ...draft.ai, enabled: v } })} />)}
              {draft.ai.enabled && (
                <>
                  {row('ai-provider', 'Leverandør', '', (
                    <select value={draft.ai.provider} onChange={(e) => void save({ ...draft, ai: { ...draft.ai, provider: e.target.value } })}>
                      <option value="gemini">Google Gemini (gratisnivå)</option>
                      <option value="groq">Groq (gratisnivå)</option>
                      <option value="openrouter">OpenRouter</option>
                      <option value="openai">OpenAI</option>
                      <option value="ollama">Ollama (på denne PC-en)</option>
                    </select>
                  ), 1)}
                  {row('ai-key', 'API-nøkkel', 'Blir på denne PC-en.', <input type="password" value={draft.ai.apiKey} onChange={(e) => setDraft({ ...draft, ai: { ...draft.ai, apiKey: e.target.value } })} onBlur={() => void save(draft)} />, 2)}
                </>
              )}
            </div>
          </SettingGroup>
        )}
      </section>

      {clearing && (
        <Confirm title="Slette all registrert aktivitet?" body="Spilletid, streaks og kalenderhistorikk slettes. Spill og innstillinger beholdes." confirmLabel="Slett" danger onCancel={() => setClearing(false)} onConfirm={async () => { setClearing(false); await api.clearActivity(); onToast('Aktivitetshistorikken er slettet'); }} />
      )}
    </div>
  );
}
