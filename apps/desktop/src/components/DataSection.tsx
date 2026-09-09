import { useCallback, useEffect, useState } from 'react';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type Backup, type DataFolders, type RecoveryNote } from '../api';
import { Confirm, SettingGroup, SettingRow, useBusy } from '../ui';
import { formatBytes, formatDateTime } from '../format';

function describe(reason: string): string {
  if (reason.startsWith('update-from-')) {
    const from = reason.slice('update-from-'.length);
    return from === 'unknown' ? 'Før en oppdatering' : `Før oppdatering fra ${from}`;
  }
  if (reason === 'before-restore') return 'Før en gjenoppretting';
  if (reason === 'manual') return 'Tatt av deg';
  return reason;
}

/** Your data — where it is, what has been backed up, and how to get it back. */
export function DataSection({ onToast }: { onToast: (title: string, body?: string) => void }) {
  const [backups, setBackups] = useState<Backup[]>([]);
  const [recoveries, setRecoveries] = useState<RecoveryNote[]>([]);
  const [folders, setFolders] = useState<DataFolders>({ data: '', backups: '' });
  const [restoring, setRestoring] = useState<Backup | null>(null);
  const { busy, run } = useBusy();

  const load = useCallback(() => {
    void api.listBackups().then(setBackups);
    void api.getRecoveries().then(setRecoveries);
    void api.dataFolder().then(setFolders);
  }, []);
  useEffect(load, [load]);

  const restore = (backup: Backup) =>
    run(async () => {
      try {
        const files = await api.restoreBackup(backup.id);
        onToast('Dataene er hentet tilbake', `${files.length} filer`);
        load();
      } catch (error) {
        onToast('Gjenopprettingen mislyktes', String(error));
      }
    });

  return (
    <>
      {recoveries.length > 0 && (
        <div className="notice">
          <strong>En fil måtte hentes fra sikkerhetskopi</strong>
          {recoveries.map((note) => (
            <p key={note.file} style={{ margin: '4px 0 0' }}>
              {note.file}{' '}
              {note.restored
                ? 'kunne ikke leses, så den nyeste sikkerhetskopien ble lagt tilbake.'
                : 'kunne ikke leses, og ingen sikkerhetskopi hadde en lesbar utgave.'}{' '}
              Originalen er ikke slettet — den ligger som <code>{note.keptAt}</code>.
            </p>
          ))}
        </div>
      )}

      <SettingGroup title="Mapper">
        <SettingRow id="data-folders" title="Datamappen" description={<code>{folders.data}</code>}>
          <button className="btn sm" onClick={() => void revealItemInDir(folders.data)}>
            Åpne
          </button>
        </SettingRow>
        <SettingRow
          title="Sikkerhetskopier"
          description={
            <>
              <code>{folders.backups}</code>
              <br />
              Ligger med vilje utenfor datamappen: en oppdatering avinstallerer den gamle versjonen først, og da kan
              Windows slette datamappen — kopiene blir stående.
            </>
          }
          index={1}
        >
          <button className="btn sm" onClick={() => void revealItemInDir(folders.backups)}>
            Åpne
          </button>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="Sikkerhetskopier">
        <SettingRow
          id="backups"
          title="Ta sikkerhetskopi nå"
          description="Tas også automatisk før en ny versjon starter for første gang. De ti nyeste beholdes."
        >
          <button
            className={`btn sm${busy ? ' busy' : ''}`}
            disabled={busy}
            onClick={() =>
              run(async () => {
                try {
                  const made = await api.backupNow();
                  onToast(made ? 'Sikkerhetskopi tatt' : 'Ingenting å kopiere ennå', made ? `${made.files.length} filer` : undefined);
                  load();
                } catch (error) {
                  onToast('Sikkerhetskopien mislyktes', String(error));
                }
              })
            }
          >
            Ta kopi
          </button>
        </SettingRow>
        {backups.length === 0 ? (
          <p className="note" style={{ padding: '10px 16px 14px' }}>
            Ingen sikkerhetskopier ennå. Den første tas automatisk neste gang du oppdaterer.
          </p>
        ) : (
          backups.map((backup) => (
            <div key={backup.id} className="backup-row">
              <div className="backup-what">
                <div>{describe(backup.reason)}</div>
                <div className="backup-when">
                  {formatDateTime(backup.takenAt)} · {backup.files.length} filer · {formatBytes(backup.bytes)}
                </div>
              </div>
              {backup.reason.startsWith('update-from') && <span className="badge-pill accent">Beholdes</span>}
              <button className="btn sm" disabled={busy} onClick={() => setRestoring(backup)}>
                Hent tilbake
              </button>
            </div>
          ))
        )}
      </SettingGroup>

      {restoring && (
        <Confirm
          title={`Hente tilbake dataene fra ${formatDateTime(restoring.takenAt)}?`}
          body="Det du har nå blir sikkerhetskopiert først, så dette kan angres."
          confirmLabel="Hent tilbake"
          onCancel={() => setRestoring(null)}
          onConfirm={() => {
            const backup = restoring;
            setRestoring(null);
            void restore(backup);
          }}
        />
      )}
    </>
  );
}
