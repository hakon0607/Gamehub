import { useState } from 'react';
import { DOCS, isFallback, legalText, type LegalDoc } from '../legal';
import { Markdown } from '../components/Markdown';
import { currentLanguage, t } from '../i18n';

/**
 * The second screen, after the language and before anything else.
 *
 * Two decisions, kept apart on purpose. Accepting the terms is one; letting
 * the app send anonymous usage statistics is the other, and it starts
 * unticked. Bundling them would make the consent invalid under the GDPR,
 * and the app has to work identically for someone who says no — so nothing
 * here nags, and "Continue" is the same button either way.
 */
export function LegalGate({
  returning,
  onDone,
}: {
  /** True when the app is asking again because the documents changed. */
  returning: boolean;
  onDone: (stats: boolean) => void;
}) {
  const [stats, setStats] = useState(false);
  const [reading, setReading] = useState<LegalDoc | null>(null);
  const language = currentLanguage();

  if (reading) {
    return (
      <div className="onboarding">
        <div className="onboarding-inner legal-reader">
          <button className="btn sm btn-ghost" onClick={() => setReading(null)}>
            ← {t('legal.back')}
          </button>
          {isFallback(reading, language) && <p className="note legal-fallback">{t('legal.english_only')}</p>}
          <div className="legal-scroll">
            <Markdown text={legalText(reading, language)} />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="onboarding">
      <div className="onboarding-inner" style={{ width: 'min(680px, 100%)' }}>
        <div className="brand" style={{ justifyContent: 'center', fontSize: 20 }}>
          <span className="brand-mark">◆</span> GameHub
        </div>
        <h1 style={{ marginBottom: 6 }}>{returning ? t('legal.gate_title_again') : t('legal.gate_title')}</h1>
        <p className="note" style={{ margin: '0 auto 20px' }}>
          {returning ? t('legal.gate_blurb_again') : t('legal.gate_blurb')}
        </p>

        <div className="legal-summary">
          <p>{t('legal.summary_local')}</p>
          <p>{t('legal.summary_free')}</p>
          <p>{t('legal.summary_anticheat')}</p>
        </div>

        <div className="legal-links">
          {DOCS.slice(0, 2).map((doc) => (
            <button key={doc.id} className="btn sm" onClick={() => setReading(doc.id)}>
              {t(doc.key as never)}
            </button>
          ))}
        </div>

        <label className="legal-consent">
          <input type="checkbox" checked={stats} onChange={(e) => setStats(e.target.checked)} />
          <span>
            <strong>{t('legal.consent_title')}</strong>
            <small>{t('legal.consent_body')}</small>
          </span>
        </label>

        <button className="btn btn-accent big" onClick={() => onDone(stats)} autoFocus>
          {t('legal.accept')} →
        </button>
        <p className="note legal-foot">{t('legal.accept_hint')}</p>
      </div>
    </div>
  );
}
