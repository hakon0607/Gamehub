import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type FreezePoint, type FreezeStatus } from '../api';
import { formatBytes, formatDateTime, formatRelativeDay } from '../format';
import { Confirm, Kbd, PageHead } from '../ui';

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
      onToast('Lagringen er lagt tilbake', await api.restoreFreezeSave(point.id));
    } catch (error) {
      onToast('Kunne ikke gjenopprette', String(error));
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
      onToast('Notatet ble ikke lagret', String(error));
    }
  };

  return (
    <div className="view">
      <PageHead
        title="Frys spillet"
        blurb={
          <>
            Trykk <Kbd>{freezeHotkey}</Kbd> midt i en cutscene, en dialog eller et oppdrag der spillet selv ikke lar deg
            lagre. Spillet stopper der det står — hvert bilde, hver timer — og fortsetter fra nøyaktig samme sted når
            du vil. Samtidig kopieres lagringsmappen, så du har et punkt å gå tilbake til.
          </>
        }
      >
        {active ? (
          <button className="btn btn-accent" onClick={() => onResume(active.id)}>
            ▶ Fortsett {active.gameName}
          </button>
        ) : (
          <button className="btn btn-accent" onClick={onFreeze} disabled={!supported || !status?.currentGameId}>
            ❄ Frys {status?.currentGameName ?? 'spillet'} nå
          </button>
        )}
      </PageHead>

      {!supported && (
        <div className="notice danger">Frysing er ikke tilgjengelig på denne maskinen. Windows tilbyr ikke suspendering av prosesser her.</div>
      )}

      {active && (
        <div className="panel glow" style={{ marginBottom: 20 }}>
          <div className="row" style={{ justifyContent: 'space-between' }}>
            <h3 className="section-title" style={{ margin: 0 }}>
              <span className="pulse frozen" /> {active.gameName} er frosset
            </h3>
            <span className="note" style={{ fontSize: 12 }}>
              siden {formatDateTime(active.frozenAt)}
            </span>
          </div>
          <p className="note" style={{ marginTop: 8 }}>
            Spillet får ingen prosessortid før du fortsetter. Skjermen kan stå på det siste bildet — trykk{' '}
            <Kbd>Win</Kbd> eller <Kbd>Alt</Kbd>+<Kbd>Tab</Kbd> for å komme til skrivebordet. Frysen varer så lenge PC-en
            er på; en omstart avslutter spillet, men lagringskopien blir liggende.
          </p>
        </div>
      )}

      {!active && status?.currentGameName && supported && (
        <div className="notice info">
          {status.currentGameName} kjører.{' '}
          {status.saveDir
            ? `Lagringsmappen er kjent, så en frys tar også kopi av lagringen.`
            : `Ingen lagringsmappe er valgt — åpne spillsiden og trykk «Finn» for at frysen også skal kopiere lagringen.`}
        </div>
      )}

      {points.length === 0 ? (
        <p className="empty">
          Ingen frysepunkter ennå. Start et spill, og trykk <Kbd>{freezeHotkey}</Kbd> når du trenger en pause spillet
          ikke vil gi deg.
        </p>
      ) : (
        points.map((point, index) => (
          <div key={point.id} className={`freeze-card${point.state === 'frozen' ? ' frozen' : ''}`} style={{ ['--i' as string]: index }}>
            <div className="freeze-pic">{point.picture ? <img src={convertFileSrc(point.picture)} alt="" loading="lazy" /> : '❄'}</div>
            <div style={{ minWidth: 0 }}>
              <div className="row" style={{ justifyContent: 'space-between' }}>
                <strong style={{ fontSize: 15 }}>{point.gameName}</strong>
                <span className={`badge-pill ${point.state === 'frozen' ? 'accent' : point.state === 'resumed' ? 'ok' : ''}`}>
                  {point.state === 'frozen' ? '❄ Frosset nå' : point.state === 'resumed' ? 'Gjenopptatt' : 'Spillet er avsluttet'}
                </span>
              </div>
              <p className="note" style={{ margin: '4px 0 8px' }}>
                {formatDateTime(point.frozenAt)} · {formatRelativeDay(point.frozenAt)}
                {point.saveCopy
                  ? ` · lagringskopi: ${point.saveFiles} filer, ${formatBytes(point.saveBytes)}`
                  : ' · ingen lagringskopi (ingen lagringsmappe var valgt)'}
              </p>
              {point.skipped.length > 0 && (
                <p className="note" style={{ fontSize: 12, color: 'var(--warn)', marginBottom: 8 }}>
                  {point.skipped.length} filer ble hoppet over fordi de var for store.
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
                    placeholder="F.eks. «rett før bossen»"
                    onChange={(e) => setNoteDraft(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') void saveNote(point.id);
                      if (e.key === 'Escape') setEditing(null);
                    }}
                    style={{ flex: 1 }}
                  />
                  <button className="btn sm btn-accent" onClick={() => void saveNote(point.id)}>
                    Lagre
                  </button>
                </div>
              ) : null}
              <div className="row">
                {point.state === 'frozen' && (
                  <button className="btn sm btn-accent" onClick={() => onResume(point.id)}>
                    ▶ Fortsett
                  </button>
                )}
                {point.saveCopy && (
                  <button className="btn sm" disabled={busy} onClick={() => setRestoring(point)}>
                    Legg tilbake lagringen
                  </button>
                )}
                {point.gameId && (
                  <button className="btn sm btn-ghost" onClick={() => onOpenGame(point.gameId!)}>
                    Spillsiden
                  </button>
                )}
                <button className="btn sm btn-ghost" onClick={() => void revealItemInDir(point.folder)}>
                  Åpne mappen
                </button>
                <button
                  className="btn sm btn-ghost"
                  onClick={() => {
                    setNoteDraft(point.note);
                    setEditing(point.id);
                  }}
                >
                  Notat
                </button>
                {point.state !== 'frozen' && (
                  <button className="btn sm btn-ghost btn-danger" onClick={() => setDeleting(point)}>
                    Slett
                  </button>
                )}
              </div>
            </div>
          </div>
        ))
      )}

      {deleting && (
        <Confirm
          title={`Slette frysepunktet fra ${formatDateTime(deleting.frozenAt)}?`}
          body="Lagringskopien og bildet slettes fra disken. Spillets egen lagring rører det ikke."
          confirmLabel="Slett"
          danger
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            try {
              await api.deleteFreeze(deleting.id);
            } catch (error) {
              onToast('Kunne ikke slette', String(error));
            }
            setDeleting(null);
            onChanged();
          }}
        />
      )}

      {restoring && (
        <Confirm
          title={`Legge tilbake lagringen fra ${formatDateTime(restoring.frozenAt)}?`}
          body="Spillet må være avsluttet. Det som ligger i lagringsmappen nå kopieres til side først, så dette kan angres."
          confirmLabel="Legg tilbake"
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
