import { useMemo, useState } from 'react';
import type { Game } from '@gamehub/shared';
import type { ActivitySummary, FreezeStatus, ReplayStatus } from '../api';
import { GameCard } from '../components/GameCard';
import { formatDuration, formatRelativeDay, formatSeconds } from '../format';
import { t, tn } from '../i18n';
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
  const sessions = activity?.today.sessionCount ?? 0;

  return (
    <div className="view">
      <div className="widgets">
        {current && (
          <div className="widget now-playing" style={{ gridColumn: 'span 2' }}>
            <h3>
              <span className={`pulse${frozenNow ? ' frozen' : ''}`} />
              {frozenNow ? t('home.frozen') : t('home.now_playing')}
            </h3>
            <p className="big" style={{ fontSize: 22 }}>
              {current.gameName}
            </p>
            <p className="sub">
              {t('home.this_session', { time: formatDuration(current.seconds) })}
              {!frozenNow && !activity?.currentActive && <span className="badge-pill" style={{ marginLeft: 8 }}>{t('home.in_background')}</span>}
            </p>
            <div className="row" style={{ marginTop: 12 }}>
              <button className="btn sm" onClick={() => onOpen(current.gameId)}>
                {t('home.open_game')}
              </button>
              <button className={`btn sm${frozenNow ? ' btn-accent' : ''}`} onClick={onFreeze} disabled={!freeze?.supported}>
                {frozenNow ? t('home.resume') : t('home.freeze_now')}
              </button>
              {replay?.running && (
                <button className="btn sm" onClick={onSaveClip}>
                  {t('home.save_clip')}
                </button>
              )}
            </div>
          </div>
        )}

        <Stat
          index={1}
          label={t('home.today')}
          value={formatDuration(activity?.today.seconds ?? 0)}
          sub={`${tn('home.session', 'home.sessions', sessions)}${activity?.today.games[0] ? t('home.mostly', { name: activity.today.games[0][0] }) : ''}`}
        />
        <Stat
          index={2}
          label={t('home.streak')}
          value={
            <>
              <span className="flame">🔥</span> {activity?.streaks.current ?? 0}
            </>
          }
          sub={t('home.longest', { n: activity?.streaks.longest ?? 0 })}
        />
        <Stat index={3} label={t('home.library')} value={games.length} sub={t('home.installed_count', { n: games.filter((g) => g.installed).length })} />
        <button className="widget" style={{ textAlign: 'left', ['--i' as string]: 4 }} onClick={() => onGo('clips')}>
          <h3>
            <span className={`pulse${replay?.running ? ' rec' : ''}`} style={replay?.running ? {} : { animation: 'none', background: 'var(--ink-faint)' }} />
            {t('home.replay')}
          </h3>
          <p className="big" style={{ fontSize: 22 }}>
            {replay?.running ? t('home.replay_recording') : replay?.enabled ? t('home.replay_stopped') : t('home.replay_off')}
          </p>
          <p className="sub">
            {replay?.running
              ? `${t('home.replay_buffer', { time: formatSeconds(replay.bufferedSeconds) })}${replay.audio.systemAudio ? t('home.replay_with_audio') : ''}`
              : t('home.replay_click')}
          </p>
        </button>
        <Roulette games={games} onPlay={onPlay} onOpen={onOpen} />
      </div>

      {recent.length > 0 && (
        <section style={{ marginBottom: 30 }}>
          <h2 className="section-title">{t('home.recent')}</h2>
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
          <h2 className="section-title">{t('home.favorites')}</h2>
          <div className="grid">
            {favorites.map((game, index) => (
              <GameCard key={game.id} index={index} game={game} running={running.includes(game.id)} onOpen={() => onOpen(game.id)} onPlay={() => onPlay(game)} />
            ))}
          </div>
        </section>
      )}

      {games.length === 0 && <p className="empty">{t('home.empty')}</p>}
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
      <h3>{t('home.roulette')}</h3>
      {picked ? (
        <>
          <p className="big" style={{ fontSize: 18, minHeight: 24 }}>
            {picked.name}
          </p>
          <div className="row" style={{ marginTop: 10 }}>
            <button className="btn sm btn-accent" onClick={() => onPlay(picked)} disabled={spinning}>
              {t('common.play')}
            </button>
            <button className="btn sm" onClick={() => onOpen(picked.id)} disabled={spinning}>
              {t('common.details')}
            </button>
            <button className="btn sm icon" onClick={spin} aria-label={t('home.roulette_again')} disabled={spinning}>
              🎲
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="sub" style={{ marginBottom: 10 }}>
            {t('home.roulette_pool', { n: pool.length })}
          </p>
          <button className="btn sm btn-accent" style={{ width: '100%' }} onClick={spin} disabled={pool.length === 0}>
            {t('home.roulette_spin')}
          </button>
        </>
      )}
      <label style={{ display: 'flex', gap: 6, marginTop: 10, fontSize: 12, color: 'var(--ink-dim)', cursor: 'pointer' }}>
        <input type="checkbox" checked={onlyUnplayed} onChange={(e) => setOnlyUnplayed(e.target.checked)} /> {t('home.roulette_unplayed')}
      </label>
    </div>
  );
}
