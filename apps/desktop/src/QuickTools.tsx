import { useEffect, useMemo, useRef, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { api, type ClipItem, type FreezeStatus, type ReplayStatus } from './api';

/**
 * Quick Tools: a small always-on-top window over whatever is in front —
 * including a full-screen game. Everything it offers is something GameHub
 * already does; this is a faster way in, not a second implementation.
 */
interface Command {
  id: string;
  label: string;
  hint: string;
  icon: string;
  run: () => Promise<void> | void;
}

export function QuickTools() {
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [replay, setReplay] = useState<ReplayStatus | null>(null);
  const [freeze, setFreeze] = useState<FreezeStatus | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const hide = () => void getCurrentWebviewWindow().hide();

  useEffect(() => {
    inputRef.current?.focus();
    void api.getClipboard().then((items) => setClips(items.slice(0, 5)));
    void api.replayStatus().then(setReplay);
    void api.freezeStatus().then(setFreeze);
    // Refresh when shown again: the window is hidden, not closed.
    const onFocus = () => {
      void api.replayStatus().then(setReplay);
      void api.freezeStatus().then(setFreeze);
      void api.getClipboard().then((items) => setClips(items.slice(0, 5)));
      setMessage(null);
      setQuery('');
      inputRef.current?.focus();
    };
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, []);

  const commands = useMemo<Command[]>(() => {
    const active = freeze?.active?.state === 'frozen' ? freeze.active : null;
    const base: Command[] = [
      ...(active
        ? [{ id: 'resume', label: `Fortsett ${active.gameName}`, hint: 'spillet er frosset', icon: '▶', run: async () => { await api.resumeFreeze(active.id); setMessage('Spillet fortsetter'); } }]
        : freeze?.currentGameId
          ? [{ id: 'freeze', label: `Frys ${freeze.currentGameName} nå`, hint: 'stopper spillet der det står', icon: '❄', run: async () => { const p = await api.freezeNow(); setMessage(`${p.gameName} er frosset${p.saveCopy ? ' · lagringen er kopiert' : ''}`); } }]
          : []),
      ...(replay?.running
        ? [{ id: 'clip', label: 'Lagre replay-klipp', hint: `siste ${replay.bufferedSeconds} sek er i bufferet`, icon: '⏺', run: async () => { const c = await api.saveReplay(); setMessage(`Klipp lagret under ${c.gameName}`); } }]
        : [{ id: 'replay-on', label: replay?.enabled ? 'Replay er på, men tar ikke opp' : 'Slå på replay', hint: replay?.problem ?? 'begynner å ta opp skjermen', icon: '⏺', run: async () => { await api.setReplayEnabled(true); setMessage('Replay er på'); } }]),
      { id: 'screenshot', label: 'Ta screenshot', hint: 'lagres under spillet som kjører', icon: '⎙', run: async () => { const shot = await api.takeScreenshot(); setMessage(`Lagret under ${shot.gameName}`); } },
      { id: 'scan', label: 'Se etter nye spill', hint: 'skanner alle launchere', icon: '↻', run: async () => { await api.scanNow(); setMessage('Skanning ferdig'); } },
      ...clips.map((clip, index) => ({
        id: `clip-${clip.id}`,
        label: `Kopier: ${clip.text.slice(0, 48).replace(/\s+/g, ' ')}`,
        hint: index === 0 ? 'siste kopierte' : 'fra utklippstavlen',
        icon: '⎘',
        run: async () => { await writeText(clip.text); setMessage('Kopiert'); },
      })),
    ];
    const q = query.trim().toLowerCase();
    return q ? base.filter((c) => c.label.toLowerCase().includes(q)) : base;
  }, [query, clips, replay, freeze]);

  useEffect(() => setSelected(0), [query]);

  const run = async (command: Command) => {
    try {
      await command.run();
      window.setTimeout(hide, 900);
    } catch (error) {
      setMessage(String(error));
    }
  };

  return (
    <div
      className="quick"
      onKeyDown={(event) => {
        if (event.key === 'Escape') hide();
        if (event.key === 'ArrowDown') { event.preventDefault(); setSelected((s) => Math.min(commands.length - 1, s + 1)); }
        if (event.key === 'ArrowUp') { event.preventDefault(); setSelected((s) => Math.max(0, s - 1)); }
        if (event.key === 'Enter' && commands[selected]) { event.preventDefault(); void run(commands[selected]); }
      }}
    >
      <div className="quick-inner">
        <input ref={inputRef} className="quick-input" placeholder="Hva vil du gjøre?" value={query} onChange={(e) => setQuery(e.target.value)} />
        <div className="quick-list">
          {commands.length === 0 ? (
            <p className="quick-empty">Ingenting som passer «{query}»</p>
          ) : (
            commands.map((command, index) => (
              <button key={command.id} className="quick-item" aria-selected={index === selected} onMouseEnter={() => setSelected(index)} onClick={() => void run(command)}>
                <span style={{ width: 20, textAlign: 'center', color: 'var(--ink-dim)' }}>{command.icon}</span>
                {command.label}
                <span className="hint">{command.hint}</span>
              </button>
            ))
          )}
        </div>
        {message && <p className="quick-empty">{message}</p>}
      </div>
    </div>
  );
}
