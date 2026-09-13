import { useState } from 'react';
import { DEFAULT_LANGUAGE, LANGUAGES, setLanguage, t } from '../i18n';

/**
 * The first screen anyone sees: pick a language. English is preselected,
 * and the screen re-renders in whichever language is hovered or chosen so
 * the choice can be judged in its own words.
 */
export function LanguageChooser({ initial, onDone }: { initial: string; onDone: (code: string) => void }) {
  const [chosen, setChosen] = useState(LANGUAGES.some((l) => l.code === initial) ? initial : DEFAULT_LANGUAGE);
  setLanguage(chosen);

  return (
    <div className="onboarding">
      <div className="onboarding-inner" style={{ width: 'min(640px, 100%)' }}>
        <div className="brand" style={{ justifyContent: 'center', fontSize: 20 }}>
          <span className="brand-mark">◆</span> GameHub
        </div>
        <h1 style={{ marginBottom: 6 }}>{t('langpick.title')}</h1>
        <p className="note" style={{ margin: '0 auto 22px' }}>{t('langpick.blurb')}</p>
        <div className="themes" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(150px, 1fr))', textAlign: 'left', marginBottom: 22 }}>
          {LANGUAGES.map((lang, index) => (
            <button
              key={lang.code}
              className="theme-card"
              aria-pressed={chosen === lang.code}
              onClick={() => setChosen(lang.code)}
              onDoubleClick={() => onDone(lang.code)}
              style={{ ['--i' as string]: index, animation: 'fade-up 0.4s var(--ease) both', animationDelay: `${index * 40}ms` }}
            >
              <p style={{ fontSize: 15 }}>{lang.name}</p>
              <small style={{ color: 'var(--ink-faint)' }}>{lang.code.toUpperCase()}</small>
            </button>
          ))}
        </div>
        <button className="btn btn-accent big" onClick={() => onDone(chosen)} autoFocus>
          {t('langpick.continue')} →
        </button>
      </div>
    </div>
  );
}
