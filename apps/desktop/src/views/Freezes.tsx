import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type FreezePoint, type FreezeStatus } from '../api';
import { formatBytes, formatDateTime, formatRelativeDay } from '../format';
import { Confirm, PageHead } from '../ui';
import { t, tr } from '../i18n';

/**
 * Freeze points: games paused where they stand, and the save copies taken
 * with them.
 *
 * Honest about the limit, in the words on the page: a frozen game lives only
 * as long as the PC stays on. The save copy is the part that survives.
 */
export function Freezes({
  status,
  points,
  freezeHotkey,
  onFreeze,
  onResume,
  onChanged,
  onOpenGame,
  onToast,
}: {
  status: FreezeStatus | null;
  points: FreezePoint[];
  freezeHotkey: string;
  onFreeze: () => void;
  onResume: (id: string) => void;
  onChanged: () => void;
  onOpenGame: (id: string) => void;
  onToast: (title: string, body?: string) => void;
}) {
  const [deleting, setDeleting] = useState<FreezePoint | null>(null);
  const [restoring, setRestoring] = useState<FreezePoint | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [noteDraft, setNoteDraft] = useState('');
  const [busy, setBusy] = useState(false);

  const active = status?.active ?? null;
  const supported = status?.supported ?? false;

  const restore = async (point: FreezePoint) => {
    setBusy(true);
    try {
      onToast(t('freeze.restored'), tr(await api.restoreFreezeSave(point.id)));
    } catch (error) {
      onToast(t('freeze.restore_failed'), tr(error));
    } finally {
      setBusy(false);
    }
  };

  const saveNote = async (id: string) => {
    setEditing(null);
    try {
      await api.setFreezeNote(id, noteDraft);
      onChanged();
    } catch (error) {
      onToast(t('freeze.note_failed'), tr(error));
    }
  };

  return (
    <div className="view">
      <PageHead
        title={t('freeze.title')}
        blurb={t('freeze.blurb', { key: freezeHotkey })}
      >
        {active ? (
          <button className="btn btn-accent" onClick={() => onResume(active.id)}>
            {t('freeze.resume_game', { name: active.gameName })}
          </button>
        ) : (
          <button className="btn btn-accent" onClick={onFreeze} disabled={!supported || !status?.currentGameId}>
            {t('freeze.freeze_game', { name: status?.currentGameName ?? t('freeze.the_game') })}
          </button>
        )}
      </PageHead>

      {!supported && (
        <div className="notice danger">{t('freeze.unsupported')}</div>
      )}

      {active && (
        <div className="panel glow" style={{ marginBottom: 20 }}>
          <div className="row" style={{ justifyContent: 'space-between' }}>
            <h3 className="section-title" style={{ margin: 0 }}>
              <span className="pulse frozen" /> {t('freeze.is_frozen', { name: active.gameName })}
            </h3>
            <span className="note" style={{ fontSize: 12 }}>
              {t('freeze.since', { when: formatDateTime(active.frozenAt) })}
            </span>
          </div>
          <p className="note" style={{ marginTop: 8 }}>
            {t('freeze.frozen_body')}
          </p>
        </div>
      )}

      {!active && status?.currentGameName && supported && (
        <div className="notice info">
          {t('freeze.running', { name: status.currentGameName })} {status.saveDir ? t('freeze.save_known') : t('freeze.save_unknown')}
        </div>
      )}

      {points.length === 0 ? (
        <p className="empty">
          {t('freeze.empty', { key: freezeHotkey })}
        </p>
      ) : (
        points.map((point, index) => (
          <div key={point.id} className={`freeze-card${point.state === 'frozen' ? ' frozen' : ''}`} style={{ ['--i' as string]: index }}>
            <div className="freeze-pic">{point.picture ? <img src={convertFileSrc(point.picture)} alt="" loading="lazy" /> : '❄'}</div>
            <div style={{ minWidth: 0 }}>
              <div className="row" style={{ justifyContent: 'space-between' }}>
                <strong style={{ fontSize: 15 }}>{point.gameName}</strong>
                <span className={`badge-pill ${point.state === 'frozen' ? 'accent' : point.state === 'resumed' ? 'ok' : ''}`}>
                  {point.state === 'frozen' ? t('freeze.state_frozen') : point.state === 'resumed' ? t('freeze.state_resumed') : t('freeze.state_gone')}
                </span>
              </div>
              <p className="note" style={{ margin: '4px 0 8px' }}>
                {formatDateTime(point.frozenAt)} · {formatRelativeDay(point.frozenAt)}
                {point.saveCopy ? t('freeze.save_copy', { files: point.saveFiles, size: formatBytes(point.saveBytes) }) : t('freeze.no_save_copy')}
              </p>
              {point.skipped.length > 0 && (
                <p className="note" style={{ fontSize: 12, color: 'var(--warn)', marginBottom: 8 }}>
                  {t('freeze.skipped', { n: point.skipped.length })}
                </p>
              )}
              {point.note && editing !== point.id && (
                <p className="note" style={{ marginBottom: 8, color: 'var(--ink)' }}>
                  «{point.note}»
                </p>
              )}
              {editing === point.id ? (
                <div className="row" style={{ marginBottom: 8 }}>
                  <input
                    type="text"
                    autoFocus
                    value={noteDraft}
                    placeholder={t('freeze.note_placeholder')}
                    onChange={(e) => setNoteDraft(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') void saveNote(point.id);
                      if (e.key === 'Escape') setEditing(null);
                    }}
                    style={{ flex: 1 }}
                  />
                  <button className="btn sm btn-accent" onClick={() => void saveNote(point.id)}>
                    {t('common.save')}
                  </button>
                </div>
              ) : null}
              <div className="row">
                {point.state === 'frozen' && (
                  <button className="btn sm btn-accent" onClick={() => onResume(point.id)}>
                    {t('freeze.resume')}
                  </button>
                )}
                {point.saveCopy && (
                  <button className="btn sm" disabled={busy} onClick={() => setRestoring(point)}>
                    {t('freeze.restore')}
                  </button>
                )}
                {point.gameId && (
                  <button className="btn sm btn-ghost" onClick={() => onOpenGame(point.gameId!)}>
                    {t('freeze.game_page')}
                  </button>
                )}
                <button className="btn sm btn-ghost" onClick={() => void revealItemInDir(point.folder)}>
                  {t('common.open_folder')}
                </button>
                <button
                  className="btn sm btn-ghost"
                  onClick={() => {
                    setNoteDraft(point.note);
                    setEditing(point.id);
                  }}
                >
                  {t('freeze.note')}
                </button>
                {point.state !== 'frozen' && (
                  <button className="btn sm btn-ghost btn-danger" onClick={() => setDeleting(point)}>
                    {t('common.delete')}
                  </button>
                )}
              </div>
            </div>
          </div>
        ))
      )}

      {deleting && (
        <Confirm
          title={t('freeze.delete_confirm', { when: formatDateTime(deleting.frozenAt) })}
          body={t('freeze.delete_body')}
          confirmLabel={t('common.delete')}
          danger
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            try {
              await api.deleteFreeze(deleting.id);
            } catch (error) {
              onToast(t('freeze.delete_failed'), tr(error));
            }
            setDeleting(null);
            onChanged();
          }}
        />
      )}

      {restoring && (
        <Confirm
          title={t('freeze.restore_confirm', { when: formatDateTime(restoring.frozenAt) })}
          body={t('freeze.restore_body')}
          confirmLabel={t('freeze.restore')}
          onCancel={() => setRestoring(null)}
          onConfirm={() => {
            const point = restoring;
            setRestoring(null);
            void restore(point);
          }}
        />
      )}
    </div>
  );
}
