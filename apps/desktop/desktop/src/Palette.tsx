import { useEffect, useMemo, useRef, useState } from 'react';
import type { Game } from '@gamehub/shared';
import { SOURCE_LABELS } from '@gamehub/shared';
import { categories, searchSettings, type SettingsCategory } from './settingsIndex';
import { t } from './i18n';

/**
 * The command palette: Ctrl+K anywhere.
 *
 * One box that finds a game, a page, a setting or an action, so nothing in
 * GameHub is more than a few keystrokes away — and so a setting can be found
 * by what it does ("lyd", "buffer") rather than by which page it lives on.
 */
export interface Command {
  id: string;
  group: 'games' | 'pages' | 'settings' | 'actions';
  label: string;
  hint?: string;
  icon: string;
  run: () => void;
}

export function Palette({
  games,
  running,
  onClose,
  onOpenGame,
  onPlay,
  onGo,
  onSetting,
  actions,
}: {
  games: Game[];
  running: string[];
  onClose: () => void;
  onOpenGame: (id: string) => void;
  onPlay: (game: Game) => void;
  onGo: (view: string) => void;
  onSetting: (category: SettingsCategory, id: string) => void;
  actions: Command[];
}) {
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => input.current?.focus(), []);

  const commands = useMemo<Command[]>(() => {
    const q = query.trim().toLowerCase();
    const pageList: [string, string, string][] = [
      ['home', t('nav.home'), '⌂'],
      ['library', t('nav.library'), '▦'],
      ['favorites', t('nav.favorites'), '★'],
      ['clips', t('nav.replay'), '⏺'],
      ['freezes', t('nav.freezes'), '❄'],
      ['screenshots', t('nav.screenshots'), '⎙'],
      ['quests', t('nav.quests'), '◆'],
      ['streaks', t('nav.streaks'), '🔥'],
      ['calendar', t('nav.calendar'), '▤'],
      ['performance', t('nav.performance'), '◔'],
      ['clipboard', t('nav.clipboard'), '⎘'],
      ['settings', t('nav.settings'), '⚙'],
    ];
    const pages: Command[] = pageList.map(([id, label, icon]) => ({ id: `page-${id}`, group: 'pages', label, icon, run: () => onGo(id) }));
    const cats = categories();

    const matchedGames: Command[] = games
      .filter((g) => !g.hidden && (!q || g.name.toLowerCase().includes(q) || g.tags.some((t) => t.toLowerCase().includes(q))))
      .slice(0, q ? 8 : 5)
      .map((game) => ({
        id: `game-${game.id}`,
        group: 'games',
        label: game.name,
        hint: running.includes(game.id) ? t('palette.running') : SOURCE_LABELS[game.source],
        icon: '▶',
        run: () => (running.includes(game.id) || !game.installed ? onOpenGame(game.id) : onPlay(game)),
      }));

    const settings: Command[] = (q ? searchSettings(q) : []).slice(0, 8).map((hit) => ({
      id: `setting-${hit.id}`,
      group: 'settings',
      label: hit.title,
      hint: cats.find((c) => c.id === hit.category)?.label,
      icon: '⚙',
      run: () => onSetting(hit.category, hit.id),
    }));

    const matchedActions = actions.filter((a) => !q || a.label.toLowerCase().includes(q));
    const matchedPages = pages.filter((p) => !q || p.label.toLowerCase().includes(q));

    return [...matchedActions, ...matchedGames, ...settings, ...matchedPages];
  }, [query, games, running, actions, onGo, onOpenGame, onPlay, onSetting]);

  useEffect(() => setSelected(0), [query]);

  const run = (command: Command) => {
    onClose();
    command.run();
  };

  let lastGroup = '';

  return (
    <div className="scrim palette" onClick={onClose}>
      <div
        className="palette-box"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(event) => {
          if (event.key === 'Escape') onClose();
          if (event.key === 'ArrowDown') {
            event.preventDefault();
            setSelected((s) => Math.min(commands.length - 1, s + 1));
          }
          if (event.key === 'ArrowUp') {
            event.preventDefault();
            setSelected((s) => Math.max(0, s - 1));
          }
          if (event.key === 'Enter' && commands[selected]) {
            event.preventDefault();
            run(commands[selected]);
          }
        }}
      >
        <input
          ref={input}
          className="palette-input"
          placeholder={t('palette.placeholder')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="palette-list">
          {commands.length === 0 ? (
            <p className="palette-empty">{t('palette.empty', { q: query })}</p>
          ) : (
            commands.map((command, index) => {
              const header = command.group !== lastGroup ? command.group : null;
              lastGroup = command.group;
              return (
                <div key={command.id}>
                  {header && <div className="palette-group">{t(`palette.g_${header}`)}</div>}
                  <button
                    className="palette-item"
                    aria-selected={index === selected}
                    onMouseEnter={() => setSelected(index)}
                    onClick={() => run(command)}
                  >
                    <span className="p-icon">{command.icon}</span>
                    {command.label}
                    {command.hint && <span className="p-hint">{command.hint}</span>}
                  </button>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
