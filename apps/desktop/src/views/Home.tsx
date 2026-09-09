import { useMemo, useState } from 'react';
import type { Game } from '@gamehub/shared';
import type { ActivitySummary, FreezeStatus, ReplayStatus } from '../api';
import { GameCard } from '../components/GameCard';
import { formatDuration, formatRelativeDay, formatSeconds } from '../format';
import { Stat } from '../ui';

/**
 * The dashboard. Every widget reads from the same sources the rest of the app
 * uses — the library, the activity log, the recorder — so nothing here can
 * disagree with another page.
 */
export function Home({
  games,
  activity,
  running,
  replay,
  freeze,
  onOpen,
  onPlay,
  onGo,
  onFreeze,
  onSaveClip,
}: {
  games: Game[];
  activity: ActivitySummary | null;
  running: string[];
  replay: ReplayStatus | null;
  freeze: FreezeStatus | null;
  onOpen: (id: string) => void;
  onPlay: (game: Game) => void;
  onGo: (view: 'clips' | 'freezes' | 'quests') => void;
  onFreeze: () => void;
  onSaveClip: () => void;
}) {
  const byId = useMemo(() => new Map(games.map((g) => [g.id, g])), [games]);
  const recent = useMemo(
    () =>
      (activity?.recent ?? [])
        .map(([id, lastPlayed, seconds]) => ({ game: byId.get(id), lastPlayed, seconds }))
        .filter((entry): entry is { game: Game; lastPlayed: string; seconds: number } => Boolean(entry.game)),
    [activity, byId],
  );
  const current = activity?.current ?? null;
  const favorites = useMemo(() => games.filter((g) => g.favorite).slice(0, 6), [games]);
  const frozenNow = freeze?.active?.state === 'frozen';

  return (
    <div className="view">
      <div className="widgets">
        {current && (
          <div className="widget now-playing" style={{ gridColumn: 'span 2' }}>
            <h3>
              <span className={`pulse${frozenNow ? ' frozen' : ''}`} />
              {frozenNow ? 'Frosset' : 'Spiller nå'}
            </h3>
            <p className="big" style={{ fontSize: 22 }}>
              {current.gameName}
            </p>
            <p className="sub">{formatDuration(current.seconds)} denne økten</p>
            <div className="row" style={{ marginTop: 12 }}>
              <button className="btn sm" onClick={() => onOpen(current.gameId)}>
                Åpne spillet
              </button>
              <button className={`btn sm${frozenNow ? ' btn-accent' : ''}`} onClick={onFreeze} disabled={!freeze?.supported}>
                {frozenNow ? '▶ Fortsett' : '❄ Frys nå'}
              </button>
              {replay?.running && (
                <button className="btn sm" onClick={onSaveClip}>
                  ⏺ Lagre klipp
                </button>
              )}
            </div>
          </div>
        )}

        <Stat
          index={1}
          label="I dag"
          value={formatDuration(activity?.today.seconds ?? 0)}
          sub={`${activity?.today.sessionCount ?? 0} ${activity?.today.sessionCount === 1 ? 'økt' : 'økter'}${
            activity?.today.games[0] ? ` · mest ${activity.today.games[0][0]}` : ''
          }`}
        />
        <Stat
          index={2}
          label="Streak"
          value={
            <>
              <span className="flame">🔥</span> {activity?.streaks.current ?? 0}
            </>
          }
          sub={`Lengste: ${activity?.streaks.longest ?? 0} dager`}
        />
        <Stat
          index={3}
          label="Bibliotek"
          value={games.length}
          sub={`${games.filter((g) => g.installed).length} installert`}
        />
        <button className="widget" style={{ textAlign: 'left', ['--i' as string]: 4 }} onClick={() => onGo('clips')}>
          <h3>
            <span className={`pulse${replay?.running ? ' rec' : ''}`} style={replay?.running ? {} : { animation: 'none', background: 'var(--ink-faint)' }} />
            Replay
          </h3>
          <p className="big" style={{ fontSize: 22 }}>
            {replay?.running ? 'Tar opp' : replay?.enabled ? 'Stoppet' : 'Av'}
          </p>
          <p className="sub">
            {replay?.running
              ? `${formatSeconds(replay.bufferedSeconds)} i bufferet${replay.audio.systemAudio ? ' · med lyd' : ''}`
              : 'Klikk for å slå på'}
          </p>
        </button>
        <Roulette games={games} onPlay={onPlay} onOpen={onOpen} />
      </div>

      {recent.length > 0 && (
        <section style={{ marginBottom: 30 }}>
          <h2 className="section-title">Nylig spilt</h2>
          <div className="grid">
            {recent.slice(0, 6).map(({ game, lastPlayed, seconds }, index) => (
              <GameCard
                key={game.id}
                index={index}
                game={game}
                running={running.includes(game.id)}
                frozen={frozenNow && freeze?.active?.gameId === game.id}
                onOpen={() => onOpen(game.id)}
                onPlay={() => onPlay(game)}
                footer={`${formatRelativeDay(lastPlayed)} · ${formatDuration(seconds)}`}
              />
            ))}
          </div>
        </section>
      )}

      {favorites.length > 0 && (
        <section>
          <h2 className="section-title">Favoritter</h2>
          <div className="grid">
            {favorites.map((game, index) => (
              <GameCard
                key={game.id}
                index={index}
                game={game}
                running={running.includes(game.id)}
                onOpen={() => onOpen(game.id)}
                onPlay={() => onPlay(game)}
              />
            ))}
          </div>
        </section>
      )}

      {games.length === 0 && (
        <p className="empty">Ingen spill ennå. Installer noe i en launcher, så plukker GameHub det opp av seg selv.</p>
      )}
    </div>
  );
}

/** Picks a game at random from what is actually installed. */
function Roulette({ games, onPlay, onOpen }: { games: Game[]; onPlay: (game: Game) => void; onOpen: (id: string) => void }) {
  const [picked, setPicked] = useState<Game | null>(null);
  const [onlyUnplayed, setOnlyUnplayed] = useState(false);
  const [spinning, setSpinning] = useState(false);
  const pool = useMemo(() => games.filter((g) => g.installed && (!onlyUnplayed || !g.lastPlayed)), [games, onlyUnplayed]);

  const spin = () => {
    if (pool.length === 0) return setPicked(null);
    // A short flicker through names before landing, because a roulette that
    // just shows the answer is a lookup.
    setSpinning(true);
    let ticks = 0;
    const timer = window.setInterval(() => {
      setPicked(pool[Math.floor(Math.random() * pool.length)] ?? null);
      ticks += 1;
      if (ticks >= 9) {
        window.clearInterval(timer);
        setSpinning(false);
      }
    }, 70);
  };

  return (
    <div className="widget" style={{ ['--i' as string]: 5 }}>
      <h3>Hva skal jeg spille?</h3>
      {picked ? (
        <>
          <p className="big" style={{ fontSize: 18, minHeight: 24 }}>
            {picked.name}
          </p>
          <div className="row" style={{ marginTop: 10 }}>
            <button className="btn sm btn-accent" onClick={() => onPlay(picked)} disabled={spinning}>
              ▶ Spill
            </button>
            <button className="btn sm" onClick={() => onOpen(picked.id)} disabled={spinning}>
              Detaljer
            </button>
            <button className="btn sm icon" onClick={spin} aria-label="Trekk igjen" disabled={spinning}>
              🎲
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="sub" style={{ marginBottom: 10 }}>
            {pool.length} installerte spill å velge blant.
          </p>
          <button className="btn sm btn-accent" style={{ width: '100%' }} onClick={spin} disabled={pool.length === 0}>
            🎲 Trekk et spill
          </button>
        </>
      )}
      <label style={{ display: 'flex', gap: 6, marginTop: 10, fontSize: 12, color: 'var(--ink-dim)', cursor: 'pointer' }}>
        <input type="checkbox" checked={onlyUnplayed} onChange={(e) => setOnlyUnplayed(e.target.checked)} /> Bare spill jeg
        aldri har startet
      </label>
    </div>
  );
}
