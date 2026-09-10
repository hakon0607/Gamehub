import { useEffect, useMemo, useRef, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { api, type ClipItem, type FreezeStatus, type ReplayStatus } from './api';
import { setLanguage, t, tr } from './i18n';

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
  const [ready, setReady] = useState(false);

  const hide = () => void getCurrentWebviewWindow().hide();

  useEffect(() => {
    inputRef.current?.focus();
    void api.getSettings().then((s) => {
      setLanguage(s.language);
      setReady(true);
    });
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
        ? [{ id: 'resume', label: t('quick.resume', { name: active.gameName }), hint: t('quick.resume_hint'), icon: '▶', run: async () => { await api.resumeFreeze(active.id); setMessage(t('quick.resumed')); } }]
        : freeze?.currentGameId
          ? [{ id: 'freeze', label: t('quick.freeze', { name: freeze.currentGameName ?? '' }), hint: t('quick.freeze_hint'), icon: '❄', run: async () => { const p = await api.freezeNow(); setMessage(t('quick.frozen', { name: p.gameName, saved: p.saveCopy ? t('toast.frozen_saved') : '' })); } }]
          : []),
      ...(replay?.running
        ? [{ id: 'clip', label: t('quick.clip'), hint: t('quick.clip_hint', { n: replay.bufferedSeconds }), icon: '⏺', run: async () => { const c = await api.saveReplay(); setMessage(t('quick.clip_saved', { name: c.gameName })); } }]
        : [{ id: 'replay-on', label: replay?.enabled ? t('quick.replay_on_not_recording') : t('quick.replay_on'), hint: replay?.problem ? tr(replay.problem) : t('quick.replay_on_hint'), icon: '⏺', run: async () => { await api.setReplayEnabled(true); setMessage(t('quick.replay_now_on')); } }]),
      { id: 'screenshot', label: t('quick.screenshot'), hint: t('quick.screenshot_hint'), icon: '⎙', run: async () => { const shot = await api.takeScreenshot(); setMessage(t('quick.saved_under', { name: shot.gameName })); } },
      { id: 'scan', label: t('quick.scan'), hint: t('quick.scan_hint'), icon: '↻', run: async () => { await api.scanNow(); setMessage(t('quick.scan_done')); } },
      ...clips.map((clip, index) => ({
        id: `clip-${clip.id}`,
        label: t('quick.copy', { text: clip.text.slice(0, 48).replace(/\s+/g, ' ') }),
        hint: index === 0 ? t('quick.copy_last') : t('quick.copy_hint'),
        icon: '⎘',
        run: async () => { await writeText(clip.text); setMessage(t('quick.copied')); },
      })),
    ];
    const q = query.trim().toLowerCase();
    return q ? base.filter((c) => c.label.toLowerCase().includes(q)) : base;
  }, [query, clips, replay, freeze, ready]);

  useEffect(() => setSelected(0), [query]);

  const run = async (command: Command) => {
    try {
      await command.run();
      window.setTimeout(hide, 900);
    } catch (error) {
      setMessage(tr(error));
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
        <input ref={inputRef} className="quick-input" placeholder={t('quick.placeholder')} value={query} onChange={(e) => setQuery(e.target.value)} />
        <div className="quick-list">
          {commands.length === 0 ? (
            <p className="quick-empty">{t('quick.empty', { q: query })}</p>
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
