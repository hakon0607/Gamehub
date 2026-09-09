import { useEffect, useState } from 'react';
import type { Game } from '@gamehub/shared';
import { SOURCE_LABELS } from '@gamehub/shared';
import { open } from '@tauri-apps/plugin-dialog';
import { convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type FreezePoint, type SaveCandidate } from '../api';
import { CoverCropper } from '../components/CoverCropper';
import { formatBytes, formatDateTime, formatRelativeDay } from '../format';
import { Confirm, Modal, SettingGroup, SettingRow } from '../ui';

function formatPlaytime(seconds: number | null): string {
  if (!seconds) return '—';
  const hours = seconds / 3600;
  return hours >= 1 ? `${hours.toFixed(1)} timer` : `${Math.round(seconds / 60)} minutter`;
}

function launchDescription(game: Game): string {
  const launch = game.launch;
  if (!launch) return 'GameHub vet ikke hvordan dette spillet startes.';
  if (launch.kind === 'uri') return `Gjennom ${SOURCE_LABELS[game.source]} (${launch.uri})`;
  if (launch.kind === 'uwp') return 'Som Windows Store-app';
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
    const picked = await open({ multiple: false, filters: [{ name: 'Bilde', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }] });
    if (typeof picked !== 'string') return;
    try {
      setCropping(await api.readImageForCrop(picked));
    } catch (error) {
      onToast('Bildet kunne ikke åpnes', String(error));
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
      onToast('Navnet ble ikke endret', String(error));
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
      onToast(folder ? 'Lagringsmappe valgt' : 'Lagringsmappe fjernet', folder || undefined);
    } catch (error) {
      onToast('Mappen ble ikke lagret', String(error));
    }
  };

  const findSaves = async () => {
    setSearching(true);
    try {
      setCandidates(await api.findSaveFolders(game.id));
    } catch (error) {
      onToast('Søket mislyktes', String(error));
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
        ← Tilbake
      </button>
      <div className="detail">
        <div>
          <div className="detail-cover">
            {cover ? (
              <img src={convertFileSrc(cover)} alt="" />
            ) : (
              <div style={{ height: '100%', display: 'grid', placeItems: 'center', color: 'var(--ink-faint)' }}>Ingen cover</div>
            )}
          </div>
          <div className="row" style={{ marginTop: 10 }}>
            <button className="btn sm" style={{ flex: 1 }} onClick={() => void pickCover()}>
              Bytt cover…
            </button>
            {game.metadata?.provider === 'manual' && (
              <button
                className="btn sm btn-ghost"
                onClick={async () => {
                  await api.clearCustomCover(game.id);
                  onCoverChanged();
                }}
              >
                Fjern mitt
              </button>
            )}
          </div>
          {game.metadata?.provider === 'steam-guess' && (
            <p className="note" style={{ fontSize: 11.5, marginTop: 8 }}>
              Coveret er gjettet ut fra navnet. Stemmer det ikke, bytt det her.
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
            <h1 onDoubleClick={() => setRenaming(true)} title="Dobbeltklikk for å endre navn">
              {game.name}
            </h1>
          )}
          <p className="note">
            {SOURCE_LABELS[game.source]}
            {game.installed ? ' · Installert' : ' · Ikke installert'}
            {frozenNow ? ' · ❄ Frosset' : running ? ' · Kjører nå' : ''}
          </p>

          <div className="actions">
            <button className="btn btn-accent" onClick={onPlay} disabled={!game.installed || running}>
              {running ? 'Kjører' : '▶ Spill'}
            </button>
            {running &&
              (frozenNow && freeze ? (
                <button className="btn" onClick={() => onResume(freeze.id)}>
                  ▶ Fortsett spillet
                </button>
              ) : (
                <button className="btn" onClick={onFreeze}>
                  ❄ Frys nå
                </button>
              ))}
            <button className="btn" onClick={onToggleFavorite}>
              {game.favorite ? '★ Favoritt' : '☆ Favoritt'}
            </button>
            <button className="btn" onClick={() => setRenaming(true)}>
              Endre navn
            </button>
            <button
              className="btn"
              onClick={async () => {
                try {
                  await revealItemInDir(await api.openGameFolder(game.id));
                } catch (error) {
                  onToast('Mappen kunne ikke åpnes', String(error));
                }
              }}
            >
              Åpne mappen
            </button>
            <button className="btn btn-ghost btn-danger" onClick={() => setRemoving(true)}>
              Fjern
            </button>
          </div>

          <div className="facts" style={{ maxWidth: 560, marginBottom: 22 }}>
            <div className="fact">
              <span>Sist spilt</span>
              <span>{game.lastPlayed ? formatRelativeDay(game.lastPlayed) : 'Aldri'}</span>
            </div>
            <div className="fact">
              <span>Spilletid (fra launcher)</span>
              <span>{formatPlaytime(game.playtimeSeconds)}</span>
            </div>
            <div className="fact">
              <span>Størrelse</span>
              <span>{game.sizeBytes ? formatBytes(game.sizeBytes) : '—'}</span>
            </div>
            <div className="fact">
              <span>Mappe</span>
              <span style={{ wordBreak: 'break-all' }}>{game.installDir ?? '—'}</span>
            </div>
            <div className="fact">
              <span>Starter</span>
              <span style={{ wordBreak: 'break-all' }}>{launchDescription(game)}</span>
            </div>
            <div className="fact">
              <span>Oppdaget</span>
              <span>{formatDateTime(game.discoveredAt)}</span>
            </div>
          </div>

          <SettingGroup title="Frys og lagring">
            <SettingRow
              title="Lagringsmappe"
              description={
                saveDir ? (
                  <code>{saveDir}</code>
                ) : (
                  'Ikke valgt. Med en lagringsmappe tar «Frys nå» også en kopi av lagringen, som overlever en omstart.'
                )
              }
            >
              <button className={`btn sm${searching ? ' busy' : ''}`} onClick={() => void findSaves()} disabled={searching}>
                Finn
              </button>
              <button
                className="btn sm"
                onClick={async () => {
                  const picked = await open({ directory: true, multiple: false });
                  if (typeof picked === 'string') void chooseSaveDir(picked);
                }}
              >
                Velg…
              </button>
              {saveDir && (
                <button className="btn sm btn-ghost" onClick={() => void chooseSaveDir('')}>
                  Fjern
                </button>
              )}
            </SettingRow>
            <SettingRow
              title="Frysepunkter"
              description={points.length === 0 ? 'Ingen ennå. Trykk F7 midt i en cutscene.' : `${points.length} for dette spillet`}
              index={1}
            >
              <button className="btn sm" onClick={onGoFreezes}>
                Vis alle
              </button>
            </SettingRow>
          </SettingGroup>

          <SettingGroup title="Etiketter">
            <SettingRow title="Tagger" description="Kommaseparert. Brukes i søk.">
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
              onToast('Coveret ble ikke lagret', String(error));
            }
          }}
        />
      )}

      {removing && (
        <Confirm
          title={`Fjerne ${game.name} fra biblioteket?`}
          body="Spill en launcher fortsatt rapporterer blir skjult i stedet for slettet, og kan hentes tilbake fra biblioteket."
          confirmLabel="Fjern"
          danger
          onCancel={() => setRemoving(false)}
          onConfirm={async () => {
            setRemoving(false);
            try {
              onRemoved(await api.removeGame(game.id));
            } catch (error) {
              onToast('Spillet ble ikke fjernet', String(error));
            }
          }}
        />
      )}

      {candidates && (
        <Modal title="Hvor ligger lagringen?" onClose={() => setCandidates(null)} wide>
          <p className="note" style={{ marginBottom: 12 }}>
            GameHub har lett i de vanlige mappene etter noe som heter det samme som spillet. Velg den som stemmer —
            er du usikker, er den som ble endret sist som regel riktig.
          </p>
          {candidates.length === 0 ? (
            <p className="empty">Fant ingen mappe som ligner. Bruk «Velg…» og pek på den selv.</p>
          ) : (
            candidates.map((candidate, index) => (
              <div className="list-row" key={candidate.path} style={{ ['--i' as string]: index }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className="name" title={candidate.path}>
                    {candidate.path}
                  </div>
                  <div className="meta">
                    {candidate.location} · {candidate.files} filer · {formatBytes(candidate.bytes)}
                    {candidate.modifiedAt ? ` · endret ${formatRelativeDay(candidate.modifiedAt)}` : ''}
                  </div>
                </div>
                <span className={`badge-pill ${candidate.confidence >= 90 ? 'ok' : candidate.confidence >= 60 ? 'accent' : ''}`}>
                  {candidate.confidence} %
                </span>
                <button className="btn sm btn-accent" onClick={() => void chooseSaveDir(candidate.path)}>
                  Bruk denne
                </button>
              </div>
            ))
          )}
        </Modal>
      )}
    </div>
  );
}
