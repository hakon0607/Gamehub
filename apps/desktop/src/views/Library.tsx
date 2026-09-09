import { useEffect, useMemo, useState } from 'react';
import type { Game, GameSource } from '@gamehub/shared';
import { GAME_SOURCES, SOURCE_LABELS } from '@gamehub/shared';
import { open } from '@tauri-apps/plugin-dialog';
import { GameCard } from '../components/GameCard';
import { api } from '../api';
import { PageHead, Segmented } from '../ui';

type Sort = 'name' | 'recent' | 'playtime' | 'added';

export function Library({
  favorites,
  games,
  counts,
  running,
  frozenId,
  source,
  search,
  onSource,
  onOpen,
  onPlay,
  onLibraryChanged,
  onToast,
}: {
  favorites: boolean;
  games: Game[];
  counts: Map<GameSource, number>;
  running: string[];
  frozenId: string | null;
  source: GameSource | 'all';
  search: string;
  onSource: (source: GameSource | 'all') => void;
  onOpen: (id: string) => void;
  onPlay: (game: Game) => void;
  onLibraryChanged: () => void;
  onToast: (title: string, body?: string) => void;
}) {
  const [hidden, setHidden] = useState(0);
  const [sort, setSort] = useState<Sort>('name');
  const [installedOnly, setInstalledOnly] = useState(false);

  useEffect(() => {
    void api.hiddenCount().then(setHidden);
  }, [games.length]);

  const sorted = useMemo(() => {
    const list = installedOnly ? games.filter((g) => g.installed) : [...games];
    const time = (v: string | null) => (v ? new Date(v).getTime() : 0);
    switch (sort) {
      case 'recent':
        return list.sort((a, b) => time(b.lastPlayed) - time(a.lastPlayed));
      case 'playtime':
        return list.sort((a, b) => (b.playtimeSeconds ?? 0) - (a.playtimeSeconds ?? 0));
      case 'added':
        return list.sort((a, b) => time(b.discoveredAt) - time(a.discoveredAt));
      default:
        return list.sort((a, b) => a.name.localeCompare(b.name, 'nb'));
    }
  }, [games, sort, installedOnly]);

  const addGame = async () => {
    const picked = await open({ multiple: false, filters: [{ name: 'Program', extensions: ['exe'] }] });
    if (typeof picked !== 'string') return;
    try {
      const game = await api.addManualGame(picked);
      onLibraryChanged();
      onToast(`${game.name} lagt til`, 'Du kan endre navn og cover på spillsiden.');
    } catch (error) {
      onToast('Spillet ble ikke lagt til', String(error));
    }
  };

  return (
    <div className="view">
      <PageHead
        title={favorites ? 'Favoritter' : 'Bibliotek'}
        blurb={
          search
            ? `${sorted.length} treff på «${search}»`
            : `${sorted.length} spill${source !== 'all' ? ` fra ${SOURCE_LABELS[source]}` : ''}`
        }
      >
        <Segmented
          value={sort}
          onChange={setSort}
          options={[
            { value: 'name', label: 'Navn' },
            { value: 'recent', label: 'Sist spilt' },
            { value: 'playtime', label: 'Spilletid' },
            { value: 'added', label: 'Nyest' },
          ]}
        />
        <button className="btn" onClick={() => void addGame()}>
          + Legg til spill
        </button>
      </PageHead>

      <div className="filters">
        <button className="chip" aria-pressed={source === 'all'} onClick={() => onSource('all')}>
          Alle
        </button>
        {GAME_SOURCES.filter((s) => (counts.get(s) ?? 0) > 0).map((s) => (
          <button key={s} className="chip" aria-pressed={source === s} onClick={() => onSource(s)}>
            {SOURCE_LABELS[s]} <span style={{ opacity: 0.7 }}>{counts.get(s)}</span>
          </button>
        ))}
        <button className="chip" aria-pressed={installedOnly} onClick={() => setInstalledOnly((v) => !v)}>
          Bare installerte
        </button>
        {hidden > 0 && (
          <button
            className="btn sm btn-ghost"
            onClick={async () => {
              const restored = await api.restoreHidden();
              onLibraryChanged();
              onToast(`${restored} spill hentet tilbake`);
            }}
          >
            Vis {hidden} skjulte
          </button>
        )}
      </div>

      {sorted.length === 0 ? (
        <p className="empty">
          {favorites
            ? 'Ingen favoritter ennå. Trykk ★ på et spill for å legge det her.'
            : 'Ingenting her ennå. Installer et spill i en launcher så dukker det opp av seg selv — eller trykk «Legg til spill» og pek på en .exe.'}
        </p>
      ) : (
        <div className="grid">
          {sorted.map((game, index) => (
            <GameCard
              key={game.id}
              index={index}
              game={game}
              running={running.includes(game.id)}
              frozen={frozenId === game.id}
              onOpen={() => onOpen(game.id)}
              onPlay={() => onPlay(game)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
