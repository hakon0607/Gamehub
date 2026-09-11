import { useCallback, useEffect, useState } from 'react';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type Backup, type DataFolders, type RecoveryNote } from '../api';
import { Confirm, SettingGroup, SettingRow, useBusy } from '../ui';
import { formatBytes, formatDateTime } from '../format';
import { t, tr } from '../i18n';

function describe(reason: string): string {
  if (reason.startsWith('update-from-')) {
    const from = reason.slice('update-from-'.length);
    return from === 'unknown' ? t('data.before_update_unknown') : t('data.before_update', { from });
  }
  if (reason === 'before-restore') return t('data.before_restore');
  if (reason === 'manual') return t('data.manual');
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
        onToast(t('data.restored'), t('common.files', { n: files.length }));
        load();
      } catch (error) {
        onToast(t('data.restore_failed'), tr(error));
      }
    });

  return (
    <>
      {recoveries.length > 0 && (
        <div className="notice">
          <strong>{t('data.recovered_title')}</strong>
          {recoveries.map((note) => (
            <p key={note.file} style={{ margin: '4px 0 0' }}>
              {note.file}{' '}
              {note.restored ? t('data.recovered_restored') : t('data.recovered_none')} {t('data.original_kept')}{' '}
              <code>{note.keptAt}</code>.
            </p>
          ))}
        </div>
      )}

      <SettingGroup title={t('data.g_folders')}>
        <SettingRow id="data-folders" title={t('data.folder')} description={<code>{folders.data}</code>}>
          <button className="btn sm" onClick={() => void revealItemInDir(folders.data)}>
            {t('common.open')}
          </button>
        </SettingRow>
        <SettingRow
          title={t('data.backups')}
          description={
            <>
              <code>{folders.backups}</code>
              <br />
              {t('data.backups_hint')}
            </>
          }
          index={1}
        >
          <button className="btn sm" onClick={() => void revealItemInDir(folders.backups)}>
            {t('common.open')}
          </button>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title={t('data.g_backups')}>
        <SettingRow
          id="backups"
          title={t('data.backup_now')}
          description={t('data.backup_now_hint')}
        >
          <button
            className={`btn sm${busy ? ' busy' : ''}`}
            disabled={busy}
            onClick={() =>
              run(async () => {
                try {
                  const made = await api.backupNow();
                  onToast(made ? t('data.taken') : t('data.nothing'), made ? t('common.files', { n: made.files.length }) : undefined);
                  load();
                } catch (error) {
                  onToast(t('data.backup_failed'), tr(error));
                }
              })
            }
          >
            {t('data.take')}
          </button>
        </SettingRow>
        {backups.length === 0 ? (
          <p className="note" style={{ padding: '10px 16px 14px' }}>
            {t('data.none')}
          </p>
        ) : (
          backups.map((backup) => (
            <div key={backup.id} className="backup-row">
              <div className="backup-what">
                <div>{describe(backup.reason)}</div>
                <div className="backup-when">
                  {t('data.meta', { when: formatDateTime(backup.takenAt), files: backup.files.length, size: formatBytes(backup.bytes) })}
                </div>
              </div>
              {backup.reason.startsWith('update-from') && <span className="badge-pill accent">{t('data.kept')}</span>}
              <button className="btn sm" disabled={busy} onClick={() => setRestoring(backup)}>
                {t('data.restore')}
              </button>
            </div>
          ))
        )}
      </SettingGroup>

      {restoring && (
        <Confirm
          title={t('data.restore_confirm', { when: formatDateTime(restoring.takenAt) })}
          body={t('data.restore_body')}
          confirmLabel={t('data.restore')}
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
