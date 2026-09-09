import { useCallback, useEffect, useState } from 'react';
import { api, type ShortcutEntry } from '../api';
import { SettingGroup } from '../ui';

/** Turns a keydown into the accelerator string Tauri expects. */
function accelerator(event: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Super');
  const key = event.key;
  if (['Control', 'Alt', 'Shift', 'Meta', 'OS'].includes(key)) return null;
  const named = key === ' ' ? 'Space' : key.length === 1 ? key.toUpperCase() : key.charAt(0).toUpperCase() + key.slice(1);
  parts.push(named);
  return parts.join('+');
}

/** Norwegian labels for the backend's action ids. */
const LABELS: Record<string, string> = {
  open_gamehub: 'Åpne GameHub',
  quick_tools: 'Hurtigverktøy (over spillet)',
  screenshot: 'Ta screenshot',
  save_replay: 'Lagre replay-klipp',
  toggle_replay: 'Replay på/av',
  freeze_game: 'Frys spillet / fortsett',
  toggle_overlay: 'Vis ytelse mens du spiller',
  search: 'Søk',
  library: 'Gå til Bibliotek',
  quests: 'Gå til Quests',
  performance: 'Gå til Ytelse',
  rescan: 'Se etter nye spill',
  screenshots: 'Gå til Screenshots',
  clips: 'Gå til Replay',
  freezes: 'Gå til Frysepunkter',
  home: 'Gå til Hjem',
  settings: 'Gå til Innstillinger',
  favorites: 'Gå til Favoritter',
  streaks: 'Gå til Streaks',
  calendar: 'Gå til Kalender',
  clipboard: 'Gå til Utklippstavle',
};

export function shortcutLabel(entry: ShortcutEntry): string {
  return LABELS[entry.action] ?? entry.label;
}

/**
 * Every shortcut GameHub has, from the backend registry — so the list can
 * never claim a shortcut that does not exist or miss one that does.
 */
export function ShortcutCenter({
  onToast,
  filter,
}: {
  onToast: (title: string, body?: string) => void;
  /** Only show actions whose id is in this list, for the Freeze page etc. */
  filter?: string[];
}) {
  const [shortcuts, setShortcuts] = useState<ShortcutEntry[]>([]);
  const [listening, setListening] = useState<string | null>(null);

  const load = useCallback(() => void api.getShortcuts().then(setShortcuts), []);
  useEffect(load, [load]);

  useEffect(() => {
    if (!listening) return;
    const onKey = async (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === 'Escape') {
        setListening(null);
        return;
      }
      const binding = accelerator(event);
      if (!binding) return;
      const action = listening;
      setListening(null);
      try {
        await api.setShortcut(action, binding);
        load();
        window.dispatchEvent(new Event('shortcuts-changed'));
      } catch (error) {
        onToast('Hurtigtasten ble ikke satt', String(error));
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [listening, load, onToast]);

  const shown = filter ? shortcuts.filter((s) => filter.includes(s.action)) : shortcuts;
  const global = shown.filter((s) => s.scope === 'global');
  const app = shown.filter((s) => s.scope === 'app');

  const row = (shortcut: ShortcutEntry) => (
    <div className="shortcut-row" key={shortcut.action}>
      <span className="label">{shortcutLabel(shortcut)}</span>
      <span className="shortcut-scope">{shortcut.scope === 'global' ? 'Virker i spill' : 'I vinduet'}</span>
      <button
        className={`keycap${listening === shortcut.action ? ' listening' : ''}`}
        onClick={() => setListening(shortcut.action)}
      >
        {listening === shortcut.action ? 'Trykk …' : shortcut.binding || 'Ingen'}
      </button>
      {shortcut.binding !== shortcut.defaultBinding && (
        <button
          className="btn sm btn-ghost"
          onClick={async () => {
            await api.resetShortcut(shortcut.action);
            load();
            window.dispatchEvent(new Event('shortcuts-changed'));
          }}
        >
          Tilbakestill
        </button>
      )}
    </div>
  );

  return (
    <>
      {global.length > 0 && <SettingGroup title="Globale — virker mens du spiller">{global.map(row)}</SettingGroup>}
      {app.length > 0 && <SettingGroup title="Bare i GameHub-vinduet">{app.map(row)}</SettingGroup>}
    </>
  );
}
