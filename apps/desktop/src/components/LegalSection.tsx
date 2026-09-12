import { useEffect, useState } from 'react';
import { api, type LegalStatus } from '../api';
import { Confirm, SettingGroup, SettingRow, Toggle } from '../ui';
import { DOCS, isFallback, legalText, type LegalDoc } from '../legal';
import { Markdown } from './Markdown';
import { currentLanguage, locale, t } from '../i18n';

/**
 * Settings → Terms of Service.
 *
 * Everything legal in one place: what the app is allowed to do, what was
 * agreed and when, the switch for the usage statistics, and the two rights
 * the user should be able to exercise without emailing anyone — a copy of
 * their data, and deletion.
 */
export function LegalSection({ onToast }: { onToast: (title: string, body?: string) => void }) {
  const [status, setStatus] = useState<LegalStatus | null>(null);
  const [reading, setReading] = useState<LegalDoc | null>(null);
  const [busy, setBusy] = useState('');
  const [confirmDelete, setConfirmDelete] = useState(false);
  const language = currentLanguage();

  useEffect(() => {
    void api.legalStatus().then(setStatus);
  }, []);

  const when = (iso: string) =>
    iso ? new Date(iso).toLocaleString(locale(), { dateStyle: 'long', timeStyle: 'short' }) : '';

  if (reading) {
    return (
      <div className="legal-reader-panel">
        <div className="row" style={{ marginBottom: 10 }}>
          <button className="btn sm btn-ghost" onClick={() => setReading(null)}>
            ← {t('legal.back')}
          </button>
        </div>
        {isFallback(reading, language) && <p className="note legal-fallback">{t('legal.english_only')}</p>}
        <div className="legal-scroll">
          <Markdown text={legalText(reading, language)} />
        </div>
      </div>
    );
  }

  return (
    <>
      <SettingGroup title={t('legal.g_documents')}>
        <div className="legal-docs" data-setting="terms-documents">
          {DOCS.map((doc) => (
            <button key={doc.id} className="legal-doc" onClick={() => setReading(doc.id)}>
              <span className="legal-doc-icon" aria-hidden="true">
                §
              </span>
              <span>
                <strong>{t(doc.key as never)}</strong>
                <small>{t('legal.doc_open')}</small>
              </span>
            </button>
          ))}
        </div>
        {status && (
          <p className="note" style={{ padding: '0 16px 14px' }}>
            {status.acceptedVersion
              ? t('legal.accepted_on', { version: status.acceptedVersion, date: when(status.acceptedAt) })
              : t('legal.accepted_never')}
          </p>
        )}
      </SettingGroup>

      <SettingGroup title={t('legal.g_consent')}>
        <SettingRow id="stats-consent" title={t('legal.consent_title')} description={t('legal.consent_body')}>
          <Toggle
            checked={status?.statsConsent ?? false}
            disabled={!status}
            label={t('legal.consent_title')}
            onChange={async (next) => {
              const answer = await api.setStatsConsent(next);
              setStatus(answer);
              onToast(answer.statsConsent ? t('legal.consent_on') : t('legal.consent_off'));
            }}
          />
        </SettingRow>
        {status?.statsConsent && (
          <p className="note" style={{ padding: '0 16px 14px' }}>
            {t('legal.consent_since', { date: when(status.statsConsentAt) })}
            {status.installId ? ` · ${t('legal.install_id', { id: status.installId })}` : ''}
          </p>
        )}
      </SettingGroup>

      <SettingGroup title={t('legal.g_rights')}>
        <SettingRow id="export-data" title={t('legal.export_title')} description={t('legal.export_body')}>
          <button
            className={`btn sm${busy === 'export' ? ' busy' : ''}`}
            disabled={busy !== ''}
            onClick={async () => {
              setBusy('export');
              try {
                const path = await api.exportMyData();
                onToast(t('legal.export_done'), path);
              } catch (error) {
                onToast(t('legal.export_failed'), String(error));
              }
              setBusy('');
            }}
          >
            {t('legal.export_button')}
          </button>
        </SettingRow>
        <SettingRow id="delete-data" title={t('legal.forget_title')} description={t('legal.forget_body')} index={1}>
          <button
            className={`btn sm${busy === 'forget' ? ' busy' : ''}`}
            disabled={busy !== '' || !status?.statsConsent}
            onClick={async () => {
              setBusy('forget');
              const ok = await api.forgetStatistics().catch(() => false);
              setStatus(await api.legalStatus());
              onToast(ok ? t('legal.forget_done') : t('legal.forget_queued'));
              setBusy('');
            }}
          >
            {t('legal.forget_button')}
          </button>
        </SettingRow>
        <SettingRow title={t('legal.wipe_title')} description={t('legal.wipe_body')} index={2}>
          <button className="btn sm btn-danger" disabled={busy !== ''} onClick={() => setConfirmDelete(true)}>
            {t('legal.wipe_button')}
          </button>
        </SettingRow>
        {status?.dataFolder && <p className="note" style={{ padding: '0 16px 14px' }}>{t('legal.data_folder', { path: status.dataFolder })}</p>}
      </SettingGroup>

      <SettingGroup title={t('legal.g_contact')}>
        <p className="note" style={{ padding: 16, margin: 0 }}>{t('legal.contact_body')}</p>
      </SettingGroup>

      {confirmDelete && (
        <Confirm
          title={t('legal.wipe_confirm_title')}
          body={t('legal.wipe_confirm_body')}
          confirmLabel={t('legal.wipe_button')}
          danger
          onCancel={() => setConfirmDelete(false)}
          onConfirm={async () => {
            setConfirmDelete(false);
            setBusy('wipe');
            const removed = await api.deleteLocalData().catch(() => 0);
            onToast(t('legal.wipe_done', { n: removed }));
            setBusy('');
            window.dispatchEvent(new Event('shortcuts-changed'));
          }}
        />
      )}
    </>
  );
}
