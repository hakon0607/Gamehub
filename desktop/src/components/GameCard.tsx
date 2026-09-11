import type { Game } from '@gamehub/shared';
import { SOURCE_LABELS } from '@gamehub/shared';
import { convertFileSrc } from '@tauri-apps/api/core';
import { t } from '../i18n';

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
      <button className="card-open" onClick={onOpen} aria-label={t('common.open_game', { name: game.name })}>
        <span className="cover">
          {cover ? <img src={convertFileSrc(cover)} alt="" loading="lazy" /> : <span className="initials">{initials(game.name)}</span>}
          <span className="badge">{SOURCE_LABELS[game.source]}</span>
          {game.favorite && <span className="fav-mark">★</span>}
          {frozen ? (
            <span className="badge badge-frozen">{t('common.frozen')}</span>
          ) : (
            running && <span className="badge badge-running">{t('common.playing_now')}</span>
          )}
          {guessed && !running && (
            <span className="badge badge-guess" title={t('common.guessed_title')}>
              {t('common.guessed')}
            </span>
          )}
          <span className="play-scrim" aria-hidden="true" />
        </span>
        <span className="card-body">
          <span className="card-title" title={game.name}>
            {game.name}
          </span>
          <span className="card-meta">{footer ?? (game.installed ? t('common.installed') : t('common.not_installed'))}</span>
        </span>
      </button>
      <button
        className="play-pill"
        onClick={onPlay}
        disabled={!canPlay}
        aria-label={running ? t('common.game_running', { name: game.name }) : t('common.play_game', { name: game.name })}
      >
        {running ? t('common.running') : game.installed ? t('common.play') : t('common.not_installed')}
      </button>
    </div>
  );
}
