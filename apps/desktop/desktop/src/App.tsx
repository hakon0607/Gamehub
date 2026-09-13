import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Game, GameSource, LauncherStatus } from '@gamehub/shared';
import { SOURCE_LABELS } from '@gamehub/shared';
import { convertFileSrc } from '@tauri-apps/api/core';
import type { Update } from '@tauri-apps/plugin-updater';
import { api, events, type ActivitySummary, type FreezePoint, type FreezeStatus, type ReplayStatus, type Settings, type ShortcutEntry } from './api';
import { checkForUpdate, type UpdateStatus } from './updater';
import { Home } from './views/Home';
import { Library } from './views/Library';
import { GameDetail } from './views/GameDetail';
import { Quests } from './views/Quests';
import { Streaks } from './views/Streaks';
import { Calendar } from './views/Calendar';
import { Replay } from './views/Replay';
import { Freezes } from './views/Freezes';
import { Screenshots } from './views/Screenshots';
import { Performance } from './views/Performance';
import { Clipboard } from './views/Clipboard';
import { SettingsView } from './views/Settings';
import { Onboarding } from './views/Onboarding';
import { LanguageChooser } from './views/LanguageChooser';
import { UpdateDialog } from './components/UpdateDialog';
import { Toasts, type Toast } from './components/Toasts';
import { ShotPreview } from './components/ShotPreview';
import { Palette, type Command } from './Palette';
import { useAppShortcuts } from './useAppShortcuts';
import type { SettingsCategory } from './settingsIndex';
import { formatDuration } from './format';
import { currentLanguage, setLanguage, t, tr, type Key } from './i18n';

export type View =
  | 'home'
  | 'library'
  | 'favorites'
  | 'quests'
  | 'streaks'
  | 'calendar'
  | 'clips'
  | 'freezes'
  | 'screenshots'
  | 'performance'
  | 'clipboard'
  | 'settings';

/** Sidebar groups. Labels are `nav.<id>` keys, so they follow the language. */
const NAV: { title: Key; items: { id: View; icon: string }[] }[] = [
  { title: 'nav.games', items: [{ id: 'home', icon: '⌂' }, { id: 'library', icon: '▦' }, { id: 'favorites', icon: '★' }] },
  { title: 'nav.recording', items: [{ id: 'clips', icon: '⏺' }, { id: 'freezes', icon: '❄' }, { id: 'screenshots', icon: '⎙' }] },
  { title: 'nav.progress', items: [{ id: 'quests', icon: '◆' }, { id: 'streaks', icon: '🔥' }, { id: 'calendar', icon: '▤' }] },
  { title: 'nav.tools', items: [{ id: 'performance', icon: '◔' }, { id: 'clipboard', icon: '⎘' }, { id: 'settings', icon: '⚙' }] },
];

const NAV_LABEL: Record<View, Key> = {
  home: 'nav.home',
  library: 'nav.library',
  favorites: 'nav.favorites',
  clips: 'nav.replay',
  freezes: 'nav.freezes',
  screenshots: 'nav.screenshots',
  quests: 'nav.quests',
  streaks: 'nav.streaks',
  calendar: 'nav.calendar',
  performance: 'nav.performance',
  clipboard: 'nav.clipboard',
  settings: 'nav.settings',
};

export function App() {
  const [games, setGames] = useState<Game[]>([]);
  const [launchers, setLaunchers] = useState<LauncherStatus[]>([]);
  const [running, setRunning] = useState<string[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [view, setView] = useState<View>('home');
  const [selected, setSelected] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const searchBox = useRef<HTMLInputElement>(null);
  const [source, setSource] = useState<GameSource | 'all'>('all');
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [scanning, setScanning] = useState(false);
  const [activity, setActivity] = useState<ActivitySummary | null>(null);
  const [update, setUpdate] = useState<{ status: UpdateStatus; update: Update } | null>(null);
  const [replay, setReplay] = useState<ReplayStatus | null>(null);
  const [freeze, setFreeze] = useState<FreezeStatus | null>(null);
  const [freezes, setFreezes] = useState<FreezePoint[]>([]);
  const [palette, setPalette] = useState(false);
  const [settingsTarget, setSettingsTarget] = useState<{ category: SettingsCategory; id: string | null } | null>(null);
  const [shortcuts, setShortcuts] = useState<ShortcutEntry[]>([]);
  // Bumped whenever the language changes, so the whole tree re-renders in it.
  const [lang, setLang] = useState(currentLanguage());

  const toast = useCallback((title: string, body?: string) => {
    const id = `${Date.now()}-${Math.random()}`;
    setToasts((current) => [...current, { id, title, body }]);
    window.setTimeout(() => setToasts((current) => current.filter((t) => t.id !== id)), 6000);
  }, []);

  const refreshActivity = useCallback(() => void api.getActivity().then(setActivity), []);
  const refreshFreeze = useCallback(() => {
    void api.freezeStatus().then(setFreeze);
    void api.getFreezes().then(setFreezes);
  }, []);
  const refreshReplay = useCallback(() => void api.replayStatus().then(setReplay), []);
  const refreshLibrary = useCallback(() => void api.getLibrary().then(setGames), []);
  const refreshShortcuts = useCallback(() => void api.getShortcuts().then(setShortcuts), []);

  useEffect(() => {
    refreshLibrary();
    void api.getRunningGames().then(setRunning);
    void api.getSettings().then((s) => {
      setLanguage(s.language);
      setLang(currentLanguage());
      setSettings(s);
    });
    refreshActivity();
    refreshFreeze();
    refreshReplay();
    refreshShortcuts();
    window.addEventListener('shortcuts-changed', refreshShortcuts);

    const ticking = window.setInterval(() => {
      refreshActivity();
      refreshReplay();
    }, 30_000);

    const unlisteners = [
      events.onLibraryUpdated((result) => {
        setGames(result.games);
        if (result.launchers.length > 0) setLaunchers(result.launchers);
        setScanning(false);
      }),
      events.onGamesDiscovered((discovered) => {
        for (const game of discovered) toast(t('toast.game_added', { name: game.name }), t('toast.found_in', { source: SOURCE_LABELS[game.source] }));
      }),
      events.onRunningGames((ids) => {
        setRunning(ids);
        refreshActivity();
        refreshFreeze();
      }),
      events.onActivityUpdated(refreshActivity),
      // Titles and bodies from the backend are keys; the body may be plain.
      events.onToast((title, body) => toast(tr(title), tr(body))),
      events.onLibraryChanged(refreshLibrary),
      events.onArtworkUpdated(refreshLibrary),
      events.onFreezeChanged(refreshFreeze),
      events.onReplayState(refreshReplay),
      events.onSettingsUpdated((next) => {
        setLanguage(next.language);
        setLang(currentLanguage());
        setSettings(next);
        refreshReplay();
      }),
      events.onToggleOverlay(() => {
        setSelected(null);
        setView('performance');
      }),
      events.onShortcutUnavailable((_, binding) =>
        toast(t('toast.shortcut_taken'), t('toast.shortcut_taken_body', { binding })),
      ),
    ];
    return () => {
      window.clearInterval(ticking);
      window.removeEventListener('shortcuts-changed', refreshShortcuts);
      for (const pending of unlisteners) void pending.then((off) => off());
    };
  }, [toast, refreshActivity, refreshFreeze, refreshReplay, refreshLibrary, refreshShortcuts]);

  // The quiet update check: once, a few seconds after launch, only for a
  // version the user has not already put off.
  useEffect(() => {
    if (!settings) return;
    let cancelled = false;
    const timer = window.setTimeout(async () => {
      try {
        const found = await checkForUpdate();
        if (cancelled || !found.update || !found.status.available) return;
        if (found.status.version === settings.dismissedUpdateVersion) return;
        setUpdate({ status: found.status, update: found.update });
      } catch {
        // No internet, or a build without an updater key. Never fatal.
      }
    }, 8000);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings !== null]);

  // Theme and density live on the root element so every token swaps at once.
  useEffect(() => {
    if (!settings) return;
    const root = document.documentElement;
    root.dataset.theme = settings.theme && settings.theme !== 'dark' && settings.theme !== 'gamehub-dark' ? settings.theme : 'nattbla';
    root.dataset.density = settings.density || 'comfortable';
    if (settings.accent) root.style.setProperty('--accent', settings.accent);
    else root.style.removeProperty('--accent');
    root.style.setProperty('--app-background', settings.backgroundImage ? `url(${convertFileSrc(settings.backgroundImage)})` : 'none');
    root.style.setProperty('--background-dim', settings.backgroundImage ? '0.55' : '0');
  }, [settings]);

  const saveSettings = useCallback(async (next: Settings) => {
    const saved = await api.saveSettings(next);
    setLanguage(saved.language);
    setLang(currentLanguage());
    setSettings(saved);
  }, []);

  const go = useCallback((next: View) => {
    setSelected(null);
    setView(next);
    if (next !== 'settings') setSettingsTarget(null);
  }, []);

  const openSetting = useCallback((category: SettingsCategory, id: string | null) => {
    setSettingsTarget({ category, id });
    setSelected(null);
    setView('settings');
  }, []);

  const rescan = useCallback(async () => {
    setScanning(true);
    try {
      const result = await api.scanNow();
      setGames(result.games);
      if (result.launchers.length > 0) setLaunchers(result.launchers);
    } finally {
      setScanning(false);
    }
  }, []);

  const play = useCallback(
    async (game: Game) => {
      try {
        await api.launchGame(game.id);
        toast(t('toast.starting', { name: game.name }));
      } catch (error) {
        toast(t('toast.launch_failed'), tr(error));
      }
    },
    [toast],
  );

  const toggleFavorite = useCallback(async (game: Game) => {
    await api.setFavorite(game.id, !game.favorite);
    setGames((current) => current.map((g) => (g.id === game.id ? { ...g, favorite: !g.favorite } : g)));
  }, []);

  const freezeNow = useCallback(
    async (gameId?: string) => {
      try {
        const point = await api.freezeNow(gameId);
        toast(t('toast.frozen'), t('toast.frozen_body', { name: point.gameName, saved: point.saveCopy ? t('toast.frozen_saved') : '' }));
        refreshFreeze();
      } catch (error) {
        toast(t('toast.freeze_failed'), tr(error));
      }
    },
    [toast, refreshFreeze],
  );

  const resumeFreeze = useCallback(
    async (id: string) => {
      try {
        const point = await api.resumeFreeze(id);
        toast(t('toast.resumed'), point.gameName);
      } catch (error) {
        toast(t('toast.resume_failed'), tr(error));
      }
      refreshFreeze();
    },
    [toast, refreshFreeze],
  );

  const saveClip = useCallback(async () => {
    try {
      const clip = await api.saveReplay();
      toast(t('toast.clip_saved'), `${clip.gameName}${clip.hasAudio ? t('toast.clip_with_audio') : ''}`);
    } catch (error) {
      toast(t('toast.clip_failed'), tr(error));
    }
  }, [toast]);

  const rescanRef = useRef(rescan);
  rescanRef.current = rescan;

  const runShortcut = useCallback(
    (action: string) => {
      switch (action) {
        case 'search':
          go('library');
          window.setTimeout(() => searchBox.current?.focus(), 0);
          break;
        case 'library':
        case 'quests':
        case 'performance':
        case 'screenshots':
        case 'clips':
        case 'freezes':
        case 'home':
        case 'settings':
        case 'favorites':
        case 'streaks':
        case 'calendar':
        case 'clipboard':
          go(action as View);
          break;
        case 'rescan':
          void rescanRef.current();
          break;
        default:
          break;
      }
    },
    [go],
  );
  useAppShortcuts(runShortcut);

  // Ctrl+K opens the palette from anywhere.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        setPalette((open) => !open);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const visible = useMemo(() => games.filter((g) => !g.hidden), [games]);
  const counts = useMemo(() => {
    const map = new Map<GameSource, number>();
    for (const game of visible) map.set(game.source, (map.get(game.source) ?? 0) + 1);
    return map;
  }, [visible]);
  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    let list = visible;
    if (view === 'favorites') list = list.filter((g) => g.favorite);
    if (source !== 'all') list = list.filter((g) => g.source === source);
    if (query) list = list.filter((g) => g.name.toLowerCase().includes(query) || g.tags.some((t) => t.toLowerCase().includes(query)));
    return list;
  }, [visible, view, source, search]);

  const activeFreeze = freeze?.active?.state === 'frozen' ? freeze.active : null;
  const freezeHotkey = shortcuts.find((s) => s.action === 'freeze_game')?.binding || 'F7';
  const replayHotkey = shortcuts.find((s) => s.action === 'save_replay')?.binding || 'F8';
  const screenshotHotkey = shortcuts.find((s) => s.action === 'screenshot')?.binding || 'F9';
  const toggleReplayHotkey = shortcuts.find((s) => s.action === 'toggle_replay')?.binding || 'Ctrl+Shift+R';

  const paletteActions: Command[] = useMemo(
    () => [
      { id: 'act-rescan', group: 'actions', label: t('palette.rescan'), icon: '↻', hint: 'F5', run: () => void rescanRef.current() },
      { id: 'act-shot', group: 'actions', label: t('palette.screenshot'), icon: '⎙', hint: screenshotHotkey, run: () => void api.takeScreenshot().then((s) => toast(t('toast.screenshot_saved'), s.gameName)).catch((e) => toast(t('toast.screenshot_failed'), tr(e))) },
      ...(replay?.running ? [{ id: 'act-clip', group: 'actions' as const, label: t('palette.clip'), icon: '⏺', hint: replayHotkey, run: () => void saveClip() }] : []),
      ...(activeFreeze
        ? [{ id: 'act-resume', group: 'actions' as const, label: t('palette.resume', { name: activeFreeze.gameName }), icon: '▶', hint: freezeHotkey, run: () => void resumeFreeze(activeFreeze.id) }]
        : freeze?.currentGameId
          ? [{ id: 'act-freeze', group: 'actions' as const, label: t('palette.freeze', { name: freeze.currentGameName ?? '' }), icon: '❄', hint: freezeHotkey, run: () => void freezeNow() }]
          : []),
      { id: 'act-replay', group: 'actions', label: replay?.enabled ? t('palette.replay_off') : t('palette.replay_on'), icon: '⏺', run: () => void api.setReplayEnabled(!replay?.enabled).then(refreshReplay).catch((e) => toast(t('toast.replay_failed'), tr(e))) },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [replay, activeFreeze, freeze, toast, saveClip, resumeFreeze, freezeNow, refreshReplay, freezeHotkey, replayHotkey, screenshotHotkey, lang],
  );

  if (settings && !settings.languageChosen) {
    return (
      <>
        <div className="aurora"><i /><i /><i /></div>
        <LanguageChooser initial="en" onDone={(code) => void saveSettings({ ...settings, language: code, languageChosen: true })} />
      </>
    );
  }

  if (settings && !settings.onboarded) {
    return (
      <>
        <div className="aurora"><i /><i /><i /></div>
        <Onboarding
          launchers={launchers}
          gameCount={visible.length}
          scanning={scanning}
          onRescan={() => void rescan()}
          onDone={() => void saveSettings({ ...settings, onboarded: true })}
        />
      </>
    );
  }

  const selectedGame = selected ? games.find((g) => g.id === selected) ?? null : null;
  const current = activity?.current ?? null;

  return (
    <div key={lang} style={{ display: 'contents' }}>
      <div className="aurora" aria-hidden="true">
        <i />
        <i />
        <i />
      </div>
      <div className="app">
        <nav className="sidebar">
          <div className="brand">
            <span className="brand-mark">◆</span> GameHub
          </div>
          {NAV.map((group) => (
            <div className="nav-group" key={group.title}>
              <div className="nav-group-title">{t(group.title)}</div>
              {group.items.map((item) => {
                const count = item.id === 'library' ? visible.length : item.id === 'favorites' ? visible.filter((g) => g.favorite).length : null;
                return (
                  <button key={item.id} className="nav-item" aria-current={view === item.id && !selectedGame} onClick={() => go(item.id)}>
                    <span className="nav-icon" aria-hidden="true">
                      {item.icon}
                    </span>
                    {t(NAV_LABEL[item.id])}
                    {item.id === 'clips' && replay?.running ? (
                      <span className="nav-dot rec" title={t('nav.recording_now')} />
                    ) : item.id === 'freezes' && activeFreeze ? (
                      <span className="nav-dot" style={{ background: 'var(--accent-2)' }} title={t('nav.frozen_now')} />
                    ) : (
                      count !== null && count > 0 && <span className="nav-count">{count}</span>
                    )}
                  </button>
                );
              })}
            </div>
          ))}
          <div className="sidebar-footer">
            {t('nav.footer', { launchers: launchers.filter((l) => l.detected).length, games: visible.length })}
            <br />
            <span style={{ opacity: 0.7 }}>{t('nav.footer_hint')}</span>
          </div>
        </nav>

        <main className="main">
          <header className="topbar">
            <div className="search-wrap">
              <span className="search-icon" aria-hidden="true">
                ⌕
              </span>
              <input
                className="search"
                ref={searchBox}
                placeholder={t('top.search_games')}
                value={search}
                onChange={(e) => {
                  setSearch(e.target.value);
                  setSelected(null);
                  if (view !== 'library' && view !== 'favorites') setView('library');
                }}
              />
              <kbd>Ctrl+F</kbd>
            </div>
            <button className="btn btn-ghost" onClick={() => setPalette(true)} title={t('top.search_all_title')}>
              {t('top.search_all')} <kbd className="key">Ctrl+K</kbd>
            </button>
            <span style={{ flex: 1 }} />
            {current && (
              <div className="now-bar">
                <span className={`pulse${activeFreeze ? ' frozen' : ''}`} />
                <strong>{current.gameName}</strong>
                <span style={{ color: 'var(--ink-faint)' }}>{formatDuration(current.seconds)}</span>
                {activeFreeze ? (
                  <button className="btn sm btn-accent" onClick={() => void resumeFreeze(activeFreeze.id)}>
                    ▶ {t('top.resume')}
                  </button>
                ) : (
                  freeze?.supported && (
                    <button className="btn sm" onClick={() => void freezeNow()} title={t('top.freeze_title', { key: freezeHotkey })}>
                      ❄ {t('top.freeze')}
                    </button>
                  )
                )}
                {replay?.running && (
                  <button className="btn sm" onClick={() => void saveClip()} title={t('top.clip_title', { key: replayHotkey })}>
                    ⏺ {t('top.clip')}
                  </button>
                )}
              </div>
            )}
            <button className={`btn${scanning ? ' busy' : ''}`} onClick={() => void rescan()} disabled={scanning}>
              {scanning ? t('top.scanning') : `↻ ${t('top.scan')}`}
            </button>
          </header>

          <div className="content" key={selectedGame ? `game-${selectedGame.id}` : view}>
            {selectedGame ? (
              <GameDetail
                game={selectedGame}
                running={running.includes(selectedGame.id)}
                freeze={activeFreeze?.gameId === selectedGame.id ? activeFreeze : null}
                freezes={freezes}
                onBack={() => setSelected(null)}
                onPlay={() => void play(selectedGame)}
                onToggleFavorite={() => void toggleFavorite(selectedGame)}
                onRenamed={(name) => setGames((current) => current.map((g) => (g.id === selectedGame.id ? { ...g, name } : g)))}
                onCoverChanged={refreshLibrary}
                onRemoved={(outcome) => {
                  setSelected(null);
                  refreshLibrary();
                  toast(outcome === 'removed' ? t('toast.game_removed') : t('toast.game_hidden'), outcome === 'hidden' ? t('toast.game_hidden_body') : undefined);
                }}
                onFreeze={() => void freezeNow(selectedGame.id)}
                onResume={(id) => void resumeFreeze(id)}
                onGoFreezes={() => go('freezes')}
                onToast={toast}
                freezeKey={freezeHotkey}
              />
            ) : view === 'clips' ? (

              settings && <Replay settings={settings} onSettings={saveSettings} onToast={toast} onOpenSettings={() => openSetting('replay', null)} hotkey={replayHotkey} />
            ) : view === 'freezes' ? (
              <Freezes
                status={freeze}
                points={freezes}
                freezeHotkey={freezeHotkey}
                onFreeze={() => void freezeNow()}
                onResume={(id) => void resumeFreeze(id)}
                onChanged={refreshFreeze}
                onOpenGame={(id) => setSelected(id)}
                onToast={toast}
              />
            ) : view === 'screenshots' ? (
              <Screenshots onToast={toast} hotkey={screenshotHotkey} />
            ) : view === 'performance' ? (
              <Performance currentGame={current?.gameName ?? null} />
            ) : view === 'clipboard' ? (
              <Clipboard
                enabled={settings?.clipboardEnabled ?? true}
                onToast={toast}
                onToggle={(enabled) => settings && void saveSettings({ ...settings, clipboardEnabled: enabled })}
              />
            ) : view === 'quests' ? (
              <Quests />
            ) : view === 'streaks' ? (
              <Streaks activity={activity} />
            ) : view === 'calendar' ? (
              <Calendar />
            ) : view === 'settings' ? (
              <SettingsView
                settings={settings}
                onSaved={setSettings}
                onToast={toast}
                onUpdateFound={setUpdate}
                initialCategory={settingsTarget?.category}
                focusSetting={settingsTarget?.id ?? null}
                hotkeys={{ replay: replayHotkey, toggleReplay: toggleReplayHotkey, screenshot: screenshotHotkey }}
              />
            ) : view === 'home' ? (
              <Home
                games={visible}
                activity={activity}
                running={running}
                replay={replay}
                freeze={freeze}
                onOpen={setSelected}
                onPlay={play}
                onGo={go}
                onFreeze={() => (activeFreeze ? void resumeFreeze(activeFreeze.id) : void freezeNow())}
                onSaveClip={() => void saveClip()}
              />
            ) : (
              <Library
                favorites={view === 'favorites'}
                games={filtered}
                counts={counts}
                running={running}
                frozenId={activeFreeze?.gameId ?? null}
                source={source}
                search={search}
                onSource={setSource}
                onOpen={setSelected}
                onPlay={play}
                onLibraryChanged={refreshLibrary}
                onToast={toast}
              />
            )}
          </div>
        </main>

        {palette && (
          <Palette
            games={visible}
            running={running}
            onClose={() => setPalette(false)}
            onOpenGame={(id) => setSelected(id)}
            onPlay={(game) => void play(game)}
            onGo={(id) => go(id as View)}
            onSetting={(category, id) => openSetting(category, id)}
            actions={paletteActions}
          />
        )}

        {update && (
          <UpdateDialog
            status={update.status}
            update={update.update}
            onLater={(version) => {
              setUpdate(null);
              if (settings && version) void saveSettings({ ...settings, dismissedUpdateVersion: version });
            }}
            onClose={() => setUpdate(null)}
            onError={(message) => toast(t('toast.update_failed'), message)}
          />
        )}

        <Toasts toasts={toasts} />
        <ShotPreview onOpen={() => go('screenshots')} />
      </div>
    </div>
  );
}
