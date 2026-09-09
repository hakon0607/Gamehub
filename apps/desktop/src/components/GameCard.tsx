import type { Game } from '@gamehub/shared';
import { SOURCE_LABELS } from '@gamehub/shared';
import { convertFileSrc } from '@tauri-apps/api/core';

function initials(name: string): string {
  const words = name.split(/\s+/).filter(Boolean);
  if (words.length === 0) return '?';
  const first = words[0]?.[0] ?? '';
  const second = words.length > 1 ? words[1]?.[0] ?? '' : words[0]?.[1] ?? '';
  return (first + second).toUpperCase();
}

/**
 * One game in the grid. Two separate targets on purpose: the white Play pill
 * starts the game, and nothing else does; everywhere else opens its page.
 */
export function GameCard({
  game,
  running,
  frozen,
  onOpen,
  onPlay,
  footer,
  index = 0,
}: {
  game: Game;
  running: boolean;
  frozen?: boolean;
  onOpen: () => void;
  onPlay: () => void;
  footer?: string;
  index?: number;
}) {
  const cover = game.metadata?.coverPath;
  const guessed = game.metadata?.provider === 'steam-guess';
  const canPlay = game.installed && !running;

  return (
    <div className="card" style={{ ['--i' as string]: index }}>
      <button className="card-open" onClick={onOpen} aria-label={`Åpne ${game.name}`}>
        <span className="cover">
          {cover ? <img src={convertFileSrc(cover)} alt="" loading="lazy" /> : <span className="initials">{initials(game.name)}</span>}
          <span className="badge">{SOURCE_LABELS[game.source]}</span>
          {game.favorite && <span className="fav-mark">★</span>}
          {frozen ? (
            <span className="badge badge-frozen">❄ Frosset</span>
          ) : (
            running && <span className="badge badge-running">Spiller nå</span>
          )}
          {guessed && !running && (
            <span className="badge badge-guess" title="Coveret er gjettet ut fra navnet. Bytt det på spillsiden.">
              gjettet
            </span>
          )}
          <span className="play-scrim" aria-hidden="true" />
        </span>
        <span className="card-body">
          <span className="card-title" title={game.name}>
            {game.name}
          </span>
          <span className="card-meta">{footer ?? (game.installed ? 'Installert' : 'Ikke installert')}</span>
        </span>
      </button>
      <button
        className="play-pill"
        onClick={onPlay}
        disabled={!canPlay}
        aria-label={running ? `${game.name} kjører` : `Spill ${game.name}`}
      >
        {running ? 'Kjører' : game.installed ? '▶ Spill' : 'Ikke installert'}
      </button>
    </div>
  );
}
