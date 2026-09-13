import { useEffect, useState } from 'react';
import type { Game } from '@gamehub/shared';
import { SOURCE_LABELS } from '@gamehub/shared';
import { open } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type FreezePoint, type SaveCandidate } from '../api';
import { CoverCropper } from '../components/CoverCropper';
import { formatBytes, formatDateTime, formatPlaytime, formatRelativeDay } from '../format';
import { t, tr } from '../i18n';
import { Confirm, Modal, SettingGroup, SettingRow } from '../ui';

function launchDescription(game: Game): string {
  const launch = game.launch;
  if (!launch) return t('game.launch_unknown');
  if (launch.kind === 'uri') return t('game.launch_via', { source: SOURCE_LABELS[game.source], uri: launch.uri });
  if (launch.kind === 'uwp') return t('game.launch_uwp');
  return launch.path;
}

export function GameDetail({
  game,
  running,
  freeze,
  freezes,
  onBack,
  onPlay,
  onToggleFavorite,
  onRenamed,
  onCoverChanged,
  onRemoved,
  onFreeze,
  onResume,
  onGoFreezes,
  onToast,
  freezeKey,
}: {
  game: Game;
  running: boolean;
  freeze: FreezePoint | null;
  freezes: FreezePoint[];
  onBack: () => void;
  onPlay: () => void;
  onToggleFavorite: () => void;
  onRenamed: (name: string) => void;
  onCoverChanged: () => void;
  onRemoved: (outcome: string) => void;
  onFreeze: () => void;
  onResume: (id: string) => void;
  onGoFreezes: () => void;
  onToast: (title: string, body?: string) => void;
  freezeKey: string;
}) {
  const [renaming, setRenaming] = useState(false);
  const [draft, setDraft] = useState(game.name);
  const [cropping, setCropping] = useState<string | null>(null);
  const [removing, setRemoving] = useState(false);
  const [tagsDraft, setTagsDraft] = useState(game.tags.join(', '));
  const [saveDir, setSaveDir] = useState<string | null>(null);
  const [candidates, setCandidates] = useState<SaveCandidate[] | null>(null);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    setDraft(game.name);
    setTagsDraft(game.tags.join(', '));
    setCandidates(null);
    let cancelled = false;
    void api.getSaveFolder(game.id).then((folder) => {
      if (!cancelled) setSaveDir(folder);
    });
    return () => {
      cancelled = true;
    };
  }, [game]);

  const pickCover = async () => {
    const picked = await open({ multiple: false, filters: [{ name: t('game.pick_image'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }] });
    if (typeof picked !== 'string') return;
    try {
      setCropping(await api.readImageForCrop(picked));
    } catch (error) {
      onToast(t('game.image_failed'), tr(error));
    }
  };

  const rename = async () => {
    const name = draft.trim();
    setRenaming(false);
    if (!name || name === game.name) return;
    try {
      await api.renameGame(game.id, name);
      onRenamed(name);
    } catch (error) {
      onToast(t('game.rename_failed'), tr(error));
    }
  };

  const saveTags = async () => {
    const tags = tagsDraft.split(',').map((t) => t.trim()).filter(Boolean);
    if (tags.join(',') === game.tags.join(',')) return;
    await api.setTags(game.id, tags);
    onCoverChanged();
  };

  const chooseSaveDir = async (folder: string) => {
    try {
      await api.setSaveFolder(game.id, folder);
      setSaveDir(folder || null);
      setCandidates(null);
      onToast(folder ? t('game.save_dir_set') : t('game.save_dir_cleared'), folder || undefined);
    } catch (error) {
      onToast(t('game.save_dir_failed'), tr(error));
    }
  };

  const findSaves = async () => {
    setSearching(true);
    try {
      setCandidates(await api.findSaveFolders(game.id));
    } catch (error) {
      onToast(t('game.search_failed'), tr(error));
    } finally {
      setSearching(false);
    }
  };

  const cover = game.metadata?.coverPath;
  const frozenNow = freeze?.state === 'frozen';
  const points = freezes.filter((f) => f.gameId === game.id);

  return (
    <div className="view">
      <button className="back" onClick={onBack}>
        {t('game.back')}
      </button>
      <div className="detail">
        <div>
          <div className="detail-cover">
            {cover ? (
              <img src={convertFileSrc(cover)} alt="" />
            ) : (
              <div style={{ height: '100%', display: 'grid', placeItems: 'center', color: 'var(--ink-faint)' }}>{t('game.no_cover')}</div>
            )}
          </div>
          <div className="row" style={{ marginTop: 10 }}>
            <button className="btn sm" style={{ flex: 1 }} onClick={() => void pickCover()}>
              {t('game.change_cover')}
            </button>
            {game.metadata?.provider === 'manual' && (
              <button
                className="btn sm btn-ghost"
                onClick={async () => {
                  await api.clearCustomCover(game.id);
                  onCoverChanged();
                }}
              >
                {t('game.remove_cover')}
              </button>
            )}
          </div>
          {game.metadata?.provider === 'steam-guess' && (
            <p className="note" style={{ fontSize: 11.5, marginTop: 8 }}>
              {t('game.cover_guessed')}
            </p>
          )}
        </div>

        <div style={{ minWidth: 0 }}>
          {renaming ? (
            <input
              className="inline-input"
              autoFocus
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onBlur={() => void rename()}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void rename();
                if (e.key === 'Escape') {
                  setDraft(game.name);
                  setRenaming(false);
                }
              }}
            />
          ) : (
            <h1 onDoubleClick={() => setRenaming(true)} title={t('game.rename_hint')}>
              {game.name}
            </h1>
          )}
          <p className="note">
            {SOURCE_LABELS[game.source]}
            {' · '}{game.installed ? t('common.installed') : t('common.not_installed')}
            {frozenNow ? ` · ${t('common.frozen')}` : running ? t('game.running_now') : ''}
          </p>

          <div className="actions">
            <button className="btn btn-accent" onClick={onPlay} disabled={!game.installed || running}>
              {running ? t('common.running') : t('common.play')}
            </button>
            {running &&
              (frozenNow && freeze ? (
                <button className="btn" onClick={() => onResume(freeze.id)}>
                  {t('game.resume')}
                </button>
              ) : (
                <button className="btn" onClick={onFreeze}>
                  {t('game.freeze')}
                </button>
              ))}
            <button className="btn" onClick={onToggleFavorite}>
              {game.favorite ? t('game.favorite') : t('game.unfavorite')}
            </button>
            <button className="btn" onClick={() => setRenaming(true)}>
              {t('game.rename')}
            </button>
            <button
              className="btn"
              onClick={async () => {
                try {
                  await revealItemInDir(await api.openGameFolder(game.id));
                } catch (error) {
                  onToast(t('common.open_folder'), tr(error));
                }
              }}
            >
              {t('common.open_folder')}
            </button>
            <button className="btn btn-ghost btn-danger" onClick={() => setRemoving(true)}>
              {t('game.remove')}
            </button>
          </div>

          <div className="facts" style={{ maxWidth: 560, marginBottom: 22 }}>
            <div className="fact">
              <span>{t('game.last_played')}</span>
              <span>{game.lastPlayed ? formatRelativeDay(game.lastPlayed) : t('common.never')}</span>
            </div>
            <div className="fact">
              <span>{t('game.playtime')}</span>
              <span>{formatPlaytime(game.playtimeSeconds)}</span>
            </div>
            <div className="fact">
              <span>{t('game.size')}</span>
              <span>{game.sizeBytes ? formatBytes(game.sizeBytes) : '—'}</span>
            </div>
            <div className="fact">
              <span>{t('game.folder')}</span>
              <span style={{ wordBreak: 'break-all' }}>{game.installDir ?? '—'}</span>
            </div>
            <div className="fact">
              <span>{t('game.starts')}</span>
              <span style={{ wordBreak: 'break-all' }}>{launchDescription(game)}</span>
            </div>
            <div className="fact">
              <span>{t('game.discovered')}</span>
              <span>{formatDateTime(game.discoveredAt)}</span>
            </div>
          </div>

          <SettingGroup title={t('game.freeze_group')}>
            <SettingRow
              title={t('game.save_dir')}
              description={
                saveDir ? (
                  <code>{saveDir}</code>
                ) : (
                  t('game.save_dir_none')
                )
              }
            >
              <button className={`btn sm${searching ? ' busy' : ''}`} onClick={() => void findSaves()} disabled={searching}>
                {t('common.find')}
              </button>
              <button
                className="btn sm"
                onClick={async () => {
                  const picked = await open({ directory: true, multiple: false });
                  if (typeof picked === 'string') void chooseSaveDir(picked);
                }}
              >
                {t('common.choose')}
              </button>
              {saveDir && (
                <button className="btn sm btn-ghost" onClick={() => void chooseSaveDir('')}>
                  {t('common.remove')}
                </button>
              )}
            </SettingRow>
            <SettingRow
              title={t('game.freeze_points')}
              description={points.length === 0 ? t('game.freeze_points_none', { key: freezeKey }) : t('game.freeze_points_count', { n: points.length })}
              index={1}
            >
              <button className="btn sm" onClick={onGoFreezes}>
                {t('game.show_all')}
              </button>
            </SettingRow>
          </SettingGroup>

          <SettingGroup title={t('game.tags_group')}>
            <SettingRow title={t('game.tags')} description={t('game.tags_hint')}>
              <input
                type="text"
                value={tagsDraft}
                onChange={(e) => setTagsDraft(e.target.value)}
                onBlur={() => void saveTags()}
                onKeyDown={(e) => e.key === 'Enter' && void saveTags()}
              />
            </SettingRow>
          </SettingGroup>
        </div>
      </div>

      {cropping && (
        <CoverCropper
          dataUrl={cropping}
          gameName={game.name}
          onCancel={() => setCropping(null)}
          onSave={async (png) => {
            setCropping(null);
            try {
              await api.setCustomCover(game.id, png);
              onCoverChanged();
            } catch (error) {
              onToast(t('game.cover_failed'), tr(error));
            }
          }}
        />
      )}

      {removing && (
        <Confirm
          title={t('game.remove_confirm', { name: game.name })}
          body={t('game.remove_body')}
          confirmLabel={t('game.remove')}
          danger
          onCancel={() => setRemoving(false)}
          onConfirm={async () => {
            setRemoving(false);
            try {
              onRemoved(await api.removeGame(game.id));
            } catch (error) {
              onToast(t('game.remove_failed'), tr(error));
            }
          }}
        />
      )}

      {candidates && (
        <Modal title={t('game.saves_title')} onClose={() => setCandidates(null)} wide>
          <p className="note" style={{ marginBottom: 12 }}>
            {t('game.saves_blurb')}
          </p>
          {candidates.length === 0 ? (
            <p className="empty">{t('game.saves_none')}</p>
          ) : (
            candidates.map((candidate, index) => (
              <div className="list-row" key={candidate.path} style={{ ['--i' as string]: index }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className="name" title={candidate.path}>
                    {candidate.path}
                  </div>
                  <div className="meta">
                    {t('game.saves_meta', { location: candidate.location, files: candidate.files, size: formatBytes(candidate.bytes) })}
                    {candidate.modifiedAt ? t('game.saves_modified', { when: formatRelativeDay(candidate.modifiedAt) }) : ''}
                  </div>
                </div>
                <span className={`badge-pill ${candidate.confidence >= 90 ? 'ok' : candidate.confidence >= 60 ? 'accent' : ''}`}>
                  {candidate.confidence} %
                </span>
                <button className="btn sm btn-accent" onClick={() => void chooseSaveDir(candidate.path)}>
                  {t('game.saves_use')}
                </button>
              </div>
            ))
          )}
        </Modal>
      )}
    </div>
  );
}
