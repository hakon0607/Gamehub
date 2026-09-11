import { useCallback, useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, events, type Screenshot } from '../api';
import { formatRelativeDay } from '../format';
import { Confirm, PageHead } from '../ui';
import { t, tr } from '../i18n';

export function Screenshots({ onToast, hotkey }: { onToast: (title: string, body?: string) => void; hotkey: string }) {
  const [groups, setGroups] = useState<[string, Screenshot[]][]>([]);
  const [game, setGame] = useState('all');
  const [onlyFavorites, setOnlyFavorites] = useState(false);
  const [viewing, setViewing] = useState<Screenshot | null>(null);
  const [deleting, setDeleting] = useState<Screenshot | null>(null);

  const load = useCallback(() => void api.getScreenshots().then(setGroups), []);
  useEffect(() => {
    load();
    const pending = events.onScreenshotTaken(load);
    return () => {
      void pending.then((off) => off());
    };
  }, [load]);

  const shown = groups
    .filter(([name]) => game === 'all' || name === game)
    .map(([name, shots]) => [name, onlyFavorites ? shots.filter((s) => s.favorite) : shots] as const)
    .filter(([, shots]) => shots.length > 0);
  const total = groups.reduce((sum, [, shots]) => sum + shots.length, 0);

  return (
    <div className="view">
      <PageHead title={t('shots.title')} blurb={t('shots.blurb', { n: total, key: hotkey })}>
        <button
          className="btn btn-accent"
          onClick={async () => {
            try {
              const shot = await api.takeScreenshot();
              onToast(t('toast.screenshot_saved'), shot.gameName);
              load();
            } catch (error) {
              onToast(t('toast.screenshot_failed'), tr(error));
            }
          }}
        >
          {t('shots.take')}
        </button>
        <button className="btn" onClick={async () => void revealItemInDir(await api.screenshotFolder())}>
          {t('common.open_folder')}
        </button>
      </PageHead>

      <div className="filters">
        <button className="chip" aria-pressed={game === 'all'} onClick={() => setGame('all')}>{t('common.all')}</button>
        {groups.map(([name, shots]) => (
          <button key={name} className="chip" aria-pressed={game === name} onClick={() => setGame(name)}>
            {name} <span style={{ opacity: 0.7 }}>{shots.length}</span>
          </button>
        ))}
        <button className="chip" aria-pressed={onlyFavorites} onClick={() => setOnlyFavorites((v) => !v)}>{t('shots.favorites')}</button>
      </div>

      {total === 0 ? (
        <p className="empty">{t('shots.empty', { key: hotkey })}</p>
      ) : (
        shown.map(([name, shots]) => (
          <section key={name} style={{ marginBottom: 28 }}>
            <h3 className="section-title">{name} <span className="countdown">{shots.length}</span></h3>
            <div className="shots">
              {shots.map((shot, index) => (
                <figure key={shot.id} className="shot" style={{ ['--i' as string]: index }}>
                  <button onClick={() => setViewing(shot)} aria-label={t('shots.open', { name: shot.gameName })}>
                    <img src={convertFileSrc(shot.path)} alt="" loading="lazy" />
                  </button>
                  <figcaption>
                    <span className="name">{formatRelativeDay(shot.takenAt)}</span>
                    <button className="shot-star" aria-label={shot.favorite ? t('replay.unfavorite') : t('replay.favorite')} onClick={async () => { await api.setScreenshotFavorite(shot.id, !shot.favorite); load(); }}>
                      {shot.favorite ? '★' : '☆'}
                    </button>
                  </figcaption>
                </figure>
              ))}
            </div>
          </section>
        ))
      )}

      {viewing && (
        <div className="scrim" role="dialog" aria-modal="true" onClick={() => setViewing(null)}>
          <div className="viewer" onClick={(e) => e.stopPropagation()}>
            <img src={convertFileSrc(viewing.path)} alt="" />
            <div className="dialog-actions">
              <button className="btn" onClick={() => void revealItemInDir(viewing.path)}>{t('common.show_in_folder')}</button>
              <button className="btn btn-danger" onClick={() => setDeleting(viewing)}>{t('common.delete')}</button>
              <button className="btn btn-accent" onClick={() => setViewing(null)}>{t('common.close')}</button>
            </div>
          </div>
        </div>
      )}
      {deleting && (
        <Confirm title={t('shots.delete_confirm')} confirmLabel={t('common.delete')} danger onCancel={() => setDeleting(null)} onConfirm={async () => { await api.deleteScreenshot(deleting.id); setDeleting(null); setViewing(null); load(); }} />
      )}
    </div>
  );
}
