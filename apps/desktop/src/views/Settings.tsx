import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import type { Update } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type AudioOptions, type Settings } from '../api';
import { checkForUpdate, describeUpdateError, type UpdateStatus } from '../updater';
import { ShortcutCenter } from '../components/ShortcutCenter';
import { DataSection } from '../components/DataSection';
import { categories, searchSettings, type SettingsCategory } from '../settingsIndex';
import { Confirm, SettingGroup, SettingRow, Slider, Toggle } from '../ui';
import { formatSeconds } from '../format';
import { LANGUAGES, t, tr, type Key } from '../i18n';
import { Wallpapers } from '../components/Wallpapers';

const THEMES: { id: string; key: Key; swatch: string[] }[] = [
  { id: 'nattbla', key: 'settings.theme_nattbla', swatch: ['#05070d', '#0b1120', '#4f8cff'] },
  { id: 'midnight', key: 'settings.theme_midnight', swatch: ['#020308', '#070b16', '#6d8bff'] },
  { id: 'ocean', key: 'settings.theme_ocean', swatch: ['#04101a', '#0a1c2b', '#29b6e8'] },
  { id: 'purple-space', key: 'settings.theme_purple', swatch: ['#07051a', '#120e2e', '#a583ff'] },
  { id: 'neon', key: 'settings.theme_neon', swatch: ['#050b09', '#0b1712', '#2ef2a0'] },
  { id: 'minimal', key: 'settings.theme_minimal', swatch: ['#0e0e10', '#17171a', '#d8d8e0'] },
  { id: 'daylight', key: 'settings.theme_daylight', swatch: ['#eef1f7', '#ffffff', '#2456d6'] },
];

/**
 * Settings: one category at a time on the right, the list on the left, and a
 * search box that finds a setting by what it does rather than where it is.
 */
export function SettingsView({
  settings,
  onSaved,
  onToast,
  onUpdateFound,
  initialCategory,
  focusSetting,
  hotkeys,
}: {
  settings: Settings | null;
  onSaved: (settings: Settings) => void;
  onToast: (title: string, body?: string) => void;
  onUpdateFound: (found: { status: UpdateStatus; update: Update }) => void;
  initialCategory?: SettingsCategory;
  focusSetting?: string | null;
  hotkeys: { replay: string; toggleReplay: string; screenshot: string };
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

  useEffect(() => {
    if (!highlight) return;
    const el = document.querySelector(`[data-setting="${highlight}"]`);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    const timer = window.setTimeout(() => setHighlight(null), 2600);
    return () => window.clearTimeout(timer);
  }, [highlight, tab]);

  const hits = useMemo(() => searchSettings(query), [query]);
  const cats = categories(hotkeys.replay);

  if (!draft) return <p className="empty">{t('common.loading')}</p>;

  const save = async (next: Settings) => {
    setDraft(next);
    try {
      const saved = await api.saveSettings(next);
      onSaved(saved);
      setDraft(saved);
    } catch (error) {
      onToast(t('settings.not_saved'), tr(error));
    }
  };
  const replay = (patch: Partial<Settings['replay']>) => save({ ...draft, replay: { ...draft.replay, ...patch } });
  const active = cats.find((c) => c.id === tab) ?? cats[0]!;
  const row = (id: string, title: string, description: ReactNode, control: ReactNode, index = 0) => (
    <SettingRow id={id} title={title} description={description} highlight={highlight === id} index={index}>
      {control}
    </SettingRow>
  );
  const pickFolder = async () => {
    const picked = await open({ directory: true, multiple: false });
    return typeof picked === 'string' ? picked : null;
  };

  return (
    <div className="settings view">
      <nav className="settings-nav" aria-label={t('settings.title')}>
        <div className="search-wrap settings-search" style={{ maxWidth: 'none' }}>
          <span className="search-icon">⌕</span>
          <input
            className="search"
            placeholder={t('settings.find')}
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
            <p className="palette-empty">{t('settings.no_hits', { q: query })}</p>
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
                  {cats.find((c) => c.id === hit.category)?.icon}
                </span>
                <span>
                  {hit.title}
                  <small>{cats.find((c) => c.id === hit.category)?.label}</small>
                </span>
              </button>
            ))
          )
        ) : (
          cats.map((category) => (
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
            <SettingGroup title={t('settings.g_language')}>
              {row('language', t('settings.language'), t('settings.language_hint'), (
                <select value={draft.language || 'en'} onChange={(e) => void save({ ...draft, language: e.target.value, languageChosen: true })}>
                  {LANGUAGES.map((l) => (
                    <option key={l.code} value={l.code}>
                      {l.name}
                    </option>
                  ))}
                </select>
              ))}
            </SettingGroup>
            <SettingGroup title={t('settings.g_startup')}>
              {row('start-with-windows', t('settings.start_with_windows'), t('settings.start_with_windows_hint'), <Toggle checked={draft.startWithWindows} onChange={(v) => void save({ ...draft, startWithWindows: v })} />)}
              {row('minimise-to-tray', t('settings.tray'), t('settings.tray_hint'), <Toggle checked={draft.minimiseToTray} onChange={(v) => void save({ ...draft, minimiseToTray: v })} />, 1)}
            </SettingGroup>
            <SettingGroup title={t('settings.g_scan')}>
              {row('scan-interval', t('settings.scan_interval'), t('settings.scan_interval_hint'), (
                <select value={draft.scanIntervalMinutes} onChange={(e) => void save({ ...draft, scanIntervalMinutes: Number(e.target.value) })}>
                  <option value={0}>{t('settings.scan_changes')}</option>
                  <option value={15}>{t('settings.scan_15')}</option>
                  <option value={60}>{t('settings.scan_60')}</option>
                  <option value={360}>{t('settings.scan_360')}</option>
                </select>
              ))}
              {row('auto-add', t('settings.auto_add'), t('settings.auto_add_hint'), <Toggle checked={draft.autoAddNewGames} onChange={(v) => void save({ ...draft, autoAddNewGames: v })} />, 1)}
            </SettingGroup>
            <SettingGroup title={t('settings.g_popup')}>
              {row('overlay-popup', t('settings.overlay_popup'), t('settings.overlay_popup_hint'), <Toggle checked={draft.overlayPopup} onChange={(v) => void save({ ...draft, overlayPopup: v })} />)}
              {row('overlay-sound', t('settings.overlay_sound'), t('settings.overlay_sound_hint'), <Toggle checked={draft.overlaySound} onChange={(v) => void save({ ...draft, overlaySound: v })} />, 1)}
            </SettingGroup>
            <p className="note">{t('settings.popup_note')}</p>
          </>
        )}

        {tab === 'appearance' && (
          <>
            <SettingGroup title={t('settings.g_theme')}>
              <div style={{ padding: 16 }} data-setting="theme" className={highlight === 'theme' ? 'highlight' : ''}>
                <div className="themes">
                  {THEMES.map((theme) => (
                    <button key={theme.id} className="theme-card" aria-pressed={(draft.theme || 'nattbla') === theme.id} onClick={() => void save({ ...draft, theme: theme.id })}>
                      <span className="theme-swatch">
                        {theme.swatch.map((colour) => <i key={colour} style={{ background: colour }} />)}
                      </span>
                      <p>{t(theme.key)}</p>
                    </button>
                  ))}
                </div>
              </div>
            </SettingGroup>
            <SettingGroup title={t('settings.g_details')}>
              {row('accent', t('settings.accent'), t('settings.accent_hint'), (
                <>
                  <input type="color" value={draft.accent || '#4f8cff'} onChange={(e) => void save({ ...draft, accent: e.target.value })} />
                  {draft.accent && <button className="btn sm btn-ghost" onClick={() => void save({ ...draft, accent: '' })}>{t('settings.accent_reset')}</button>}
                </>
              ))}
              {row('density', t('settings.density'), t('settings.density_hint'), (
                <select value={draft.density} onChange={(e) => void save({ ...draft, density: e.target.value })}>
                  <option value="comfortable">{t('settings.density_comfortable')}</option>
                  <option value="compact">{t('settings.density_compact')}</option>
                </select>
              ), 1)}
              {row('background', t('settings.background'), draft.backgroundImage ? <code>{draft.backgroundImage}</code> : t('settings.background_hint'), (
                <>
                  <button className="btn sm" onClick={async () => {
                    const picked = await open({ multiple: false, filters: [{ name: t('game.pick_image'), extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
                    if (typeof picked === 'string') void save({ ...draft, backgroundImage: picked });
                  }}>{t('common.choose')}</button>
                  {draft.backgroundImage && <button className="btn sm btn-ghost" onClick={() => void save({ ...draft, backgroundImage: '' })}>{t('common.remove')}</button>}
                </>
              ), 2)}
            </SettingGroup>
            <SettingGroup title={t('wp.title')}>
              <div style={{ padding: 16 }} data-setting="wallpaper-desktop" className={highlight?.startsWith('wallpaper') ? 'highlight' : ''}>
                <Wallpapers onToast={onToast} />
              </div>
            </SettingGroup>
          </>
        )}

        {tab === 'replay' && (
          <>
            <SettingGroup title={t('settings.g_recording')}>
              {row('replay-enabled', t('settings.replay'), t('settings.replay_hint', { key: hotkeys.toggleReplay }), (
                <Toggle checked={draft.replay.enabled} onChange={async (v) => {
                  try { await api.setReplayEnabled(v); } catch (error) { onToast(t('toast.replay_failed'), tr(error)); return; }
                  onSaved(await api.getSettings());
                }} />
              ))}
              {row('replay-buffer', t('settings.buffer'), t('settings.buffer_hint'), (
                <Slider value={draft.replay.bufferSeconds} min={30} max={600} step={15} format={formatSeconds}
                  onCommit={(s) => s !== draft.replay.bufferSeconds && void replay({ bufferSeconds: s, saveSeconds: Math.min(draft.replay.saveSeconds, s) })} />
              ), 1)}
              {row('replay-save', t('settings.save_seconds', { key: hotkeys.replay }), t('settings.save_seconds_hint'), (
                <select value={draft.replay.saveSeconds} onChange={(e) => void replay({ saveSeconds: Number(e.target.value) })}>
                  {[15, 30, 60, 120, 180, 300, 600].filter((s) => s <= draft.replay.bufferSeconds).map((s) => <option key={s} value={s}>{formatSeconds(s)}</option>)}
                </select>
              ), 2)}
              {row('replay-folder', t('settings.clip_folder'), draft.replay.folder ? <code>{draft.replay.folder}</code> : t('settings.clip_folder_hint'), (
                <>
                  <button className="btn sm" onClick={async () => { const f = await pickFolder(); if (f) void replay({ folder: f }); }}>{t('common.choose')}</button>
                  {draft.replay.folder && <button className="btn sm btn-ghost" onClick={() => void replay({ folder: '' })}>{t('common.default')}</button>}
                </>
              ), 3)}
            </SettingGroup>
            <SettingGroup title={t('settings.g_sound')}>
              {row('replay-system-audio', t('settings.system_audio'), t('settings.system_audio_hint'), <Toggle checked={draft.replay.systemAudio} onChange={(v) => void replay({ systemAudio: v })} />)}
              {row('replay-mic', t('settings.mic'), audio.devices.length === 0 ? t('settings.mic_none_found') : t('settings.mic_hint'), (
                <select value={draft.replay.audioDevice || 'none'} onChange={(e) => void replay({ audioDevice: e.target.value === 'none' ? '' : e.target.value })}>
                  <option value="none">{t('settings.mic_none')}</option>
                  {audio.devices.map((d) => <option key={d} value={d}>{d}{d === audio.suggested ? t('settings.mic_loopback') : ''}</option>)}
                  {draft.replay.audioDevice && !audio.devices.includes(draft.replay.audioDevice) && <option value={draft.replay.audioDevice}>{t('settings.mic_missing', { name: draft.replay.audioDevice })}</option>}
                </select>
              ), 1)}
            </SettingGroup>
            <SettingGroup title={t('settings.g_picture')}>
              {row('replay-encoder', t('settings.encoder'), t('settings.encoder_hint'), (
                <select value={draft.replay.encoder} onChange={(e) => void replay({ encoder: e.target.value })}>
                  <option value="auto">{t('settings.encoder_auto')}</option>
                  <option value="cpu">{t('settings.encoder_cpu')}</option>
                </select>
              ))}
              {row('replay-scale', t('settings.scale'), t('settings.scale_hint'), (
                <select value={draft.replay.scaleHeight} onChange={(e) => void replay({ scaleHeight: Number(e.target.value) })}>
                  <option value={720}>{t('settings.scale_720')}</option>
                  <option value={1080}>{t('settings.scale_1080')}</option>
                  <option value={1440}>{t('settings.scale_1440')}</option>
                  <option value={0}>{t('settings.scale_native')}</option>
                </select>
              ), 1)}
              {row('replay-fps', t('settings.fps'), '', (
                <select value={draft.replay.fps} onChange={(e) => void replay({ fps: Number(e.target.value) })}>
                  <option value={30}>30</option>
                  <option value={60}>60</option>
                </select>
              ), 2)}
              {row('replay-quality', t('settings.quality'), t('settings.quality_hint'), (
                <select value={draft.replay.quality} onChange={(e) => void replay({ quality: e.target.value })}>
                  <option value="low">{t('settings.quality_low')}</option>
                  <option value="medium">{t('settings.quality_medium')}</option>
                  <option value="high">{t('settings.quality_high')}</option>
                </select>
              ), 3)}
            </SettingGroup>
            <p className="note">{t('settings.fullscreen_note')}</p>
          </>
        )}

        {tab === 'freeze' && (
          <>
            <SettingGroup title={t('settings.g_hotkey')}>
              <div data-setting="freeze-hotkey" className={highlight === 'freeze-hotkey' ? 'highlight' : ''}>
                <ShortcutCenter onToast={onToast} filter={['freeze_game', 'freezes']} />
              </div>
            </SettingGroup>
            <SettingGroup title={t('settings.g_saves')}>
              {row('freeze-saves', t('settings.save_per_game'), t('settings.save_per_game_hint'), <span className="badge-pill">{t('settings.on_game_page')}</span>)}
              {row('freeze-folder', t('settings.freeze_folder'), <>{t('settings.freeze_folder_hint', { path: '' })}<code>{draft.replay.folder || 'Clips'}\…\Frys_…</code></>, <button className="btn sm" onClick={() => setTab('replay')}>{t('settings.change')}</button>, 1)}
            </SettingGroup>
            <p className="note">{t('settings.freeze_note')}</p>
          </>
        )}

        {tab === 'shortcuts' && (
          <div data-setting="shortcuts">
            <p className="note" style={{ marginBottom: 14 }}>{t('settings.shortcuts_hint')}</p>
            <ShortcutCenter onToast={onToast} />
          </div>
        )}

        {tab === 'screenshots' && (
          <SettingGroup>
            {row('screenshot-monitor', t('settings.monitor'), t('settings.monitor_hint', { key: hotkeys.screenshot }), (
              <select value={draft.screenshotMonitor} onChange={(e) => void save({ ...draft, screenshotMonitor: Number(e.target.value) })}>
                {displays.length === 0 ? <option value={0}>{t('settings.monitor_primary')}</option> : displays.map((name, index) => <option key={name} value={index}>{name}</option>)}
              </select>
            ))}
            {row('screenshot-folder', t('settings.shot_folder'), draft.screenshotFolder ? <code>{draft.screenshotFolder}</code> : t('settings.shot_folder_hint'), (
              <>
                <button className="btn sm" onClick={async () => { const f = await pickFolder(); if (f) void save({ ...draft, screenshotFolder: f }); }}>{t('common.choose')}</button>
                {draft.screenshotFolder && <button className="btn sm btn-ghost" onClick={() => void save({ ...draft, screenshotFolder: '' })}>{t('common.default')}</button>}
                <button className="btn sm btn-ghost" onClick={async () => void revealItemInDir(await api.screenshotFolder())}>{t('common.open')}</button>
              </>
            ), 1)}
          </SettingGroup>
        )}

        {tab === 'folders' && (
          <SettingGroup title={t('settings.g_extra')}>
            <div data-setting="extra-folders" className={highlight === 'extra-folders' ? 'highlight' : ''}>
              <p className="note" style={{ padding: '12px 16px 4px' }}>{t('settings.extra_hint')}</p>
              {draft.extraGameFolders.map((folder, index) => (
                <SettingRow key={folder} title={folder} index={index}>
                  <button className="btn sm btn-ghost btn-danger" onClick={() => void save({ ...draft, extraGameFolders: draft.extraGameFolders.filter((f) => f !== folder) })}>{t('common.remove')}</button>
                </SettingRow>
              ))}
              <div style={{ padding: 16 }}>
                <button className="btn" onClick={async () => {
                  const picked = await pickFolder();
                  if (!picked) return;
                  const saved = await api.addGameFolder(picked);
                  onSaved(saved);
                  setDraft(saved);
                }}>{t('settings.add_folder')}</button>
              </div>
            </div>
          </SettingGroup>
        )}

        {tab === 'privacy' && (
          <>
            <SettingGroup title={t('settings.g_activity')}>
              {row('track-activity', t('settings.track'), t('settings.track_hint'), <Toggle checked={draft.trackActivity} onChange={(v) => void save({ ...draft, trackActivity: v })} />)}
              {row('active-only', t('settings.active_only'), t('settings.active_only_hint'), <Toggle checked={draft.trackActiveOnly} onChange={(v) => void save({ ...draft, trackActiveOnly: v, activeOnlyChosen: true })} />, 1)}
              {row('idle-minutes', t('settings.idle'), t('settings.idle_hint'), (
                <select value={draft.idleMinutes} onChange={(e) => void save({ ...draft, idleMinutes: Number(e.target.value) })}>
                  {[5, 10, 20, 30].map((m) => <option key={m} value={m}>{t('settings.threshold_minutes', { n: m })}</option>)}
                  <option value={0}>{t('common.never')}</option>
                </select>
              ), 2)}
              {row('streak-threshold', t('settings.threshold'), '', (
                <select value={draft.streakThresholdMinutes} onChange={(e) => void save({ ...draft, streakThresholdMinutes: Number(e.target.value) })}>
                  {[5, 15, 30, 60].map((m) => <option key={m} value={m}>{t('settings.threshold_minutes', { n: m })}</option>)}
                </select>
              ), 3)}
              {row('clipboard-enabled', t('settings.clipboard'), t('settings.clipboard_hint'), <Toggle checked={draft.clipboardEnabled} onChange={(v) => void save({ ...draft, clipboardEnabled: v })} />, 4)}
            </SettingGroup>
            <SettingGroup title={t('settings.g_delete')}>
              {row('clear-activity', t('settings.clear_activity'), t('settings.clear_activity_hint'), <button className="btn sm btn-danger" onClick={() => setClearing(true)}>{t('settings.clear_activity_button')}</button>)}
            </SettingGroup>
          </>
        )}

        {tab === 'data' && <DataSection onToast={onToast} />}

        {tab === 'updates' && (
          <SettingGroup title={t('settings.g_version', { version })}>
            {row('check-updates', t('settings.updates'), canUpdate ? t('settings.updates_hint') : t('settings.updates_local'), (
              <button className={`btn sm${checking ? ' busy' : ''}`} disabled={checking || !canUpdate} onClick={async () => {
                setChecking(true);
                try {
                  const found = await checkForUpdate();
                  if (found.update && found.status.available) onUpdateFound({ status: found.status, update: found.update });
                  else onToast(t('settings.up_to_date'), `GameHub ${found.status.currentVersion}`);
                } catch (error) {
                  onToast(t('settings.check_failed'), describeUpdateError(error));
                } finally {
                  setChecking(false);
                }
              }}>{t('settings.check_updates')}</button>
            ))}
          </SettingGroup>
        )}

        {tab === 'artwork' && (
          <>
            <SettingGroup title={t('settings.g_covers')}>
              {row('refresh-artwork', t('settings.refresh_art'), t('settings.refresh_art_hint'), (
                <button className="btn sm" onClick={async () => { await api.refreshArtwork(); onToast(t('settings.fetching_art')); }}>{t('settings.fetch_now')}</button>
              ))}
            </SettingGroup>
            <SettingGroup title={t('settings.g_keys')}>
              <div data-setting="artwork-keys" className={highlight === 'artwork-keys' ? 'highlight' : ''}>
                {row('igdb-id', t('settings.igdb_id'), '', <input type="text" value={draft.metadata.igdbClientId} onChange={(e) => setDraft({ ...draft, metadata: { ...draft.metadata, igdbClientId: e.target.value } })} onBlur={() => void save(draft)} />)}
                {row('igdb-secret', t('settings.igdb_secret'), '', <input type="password" value={draft.metadata.igdbClientSecret} onChange={(e) => setDraft({ ...draft, metadata: { ...draft.metadata, igdbClientSecret: e.target.value } })} onBlur={() => void save(draft)} />, 1)}
                {row('sgdb', t('settings.sgdb'), '', <input type="password" value={draft.metadata.steamGridDbKey} onChange={(e) => setDraft({ ...draft, metadata: { ...draft.metadata, steamGridDbKey: e.target.value } })} onBlur={() => void save(draft)} />, 2)}
              </div>
            </SettingGroup>
          </>
        )}

        {tab === 'assistant' && (
          <SettingGroup>
            <div data-setting="assistant" className={highlight === 'assistant' ? 'highlight' : ''}>
              {row('ai-enabled', t('settings.assistant'), t('settings.assistant_hint'), <Toggle checked={draft.ai.enabled} onChange={(v) => void save({ ...draft, ai: { ...draft.ai, enabled: v } })} />)}
              {draft.ai.enabled && (
                <>
                  {row('ai-provider', t('settings.provider'), '', (
                    <select value={draft.ai.provider} onChange={(e) => void save({ ...draft, ai: { ...draft.ai, provider: e.target.value } })}>
                      <option value="gemini">{t('settings.provider_gemini')}</option>
                      <option value="groq">{t('settings.provider_groq')}</option>
                      <option value="openrouter">{t('settings.provider_openrouter')}</option>
                      <option value="openai">{t('settings.provider_openai')}</option>
                      <option value="ollama">{t('settings.provider_ollama')}</option>
                    </select>
                  ), 1)}
                  {row('ai-key', t('settings.api_key'), t('settings.api_key_hint'), <input type="password" value={draft.ai.apiKey} onChange={(e) => setDraft({ ...draft, ai: { ...draft.ai, apiKey: e.target.value } })} onBlur={() => void save(draft)} />, 2)}
                </>
              )}
            </div>
          </SettingGroup>
        )}
      </section>

      {clearing && (
        <Confirm title={t('settings.clear_confirm')} body={t('settings.clear_body')} confirmLabel={t('common.delete')} danger onCancel={() => setClearing(false)} onConfirm={async () => { setClearing(false); await api.clearActivity(); onToast(t('settings.cleared')); }} />
      )}
    </div>
  );
}
