import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api } from './api';
import { t } from './i18n';

/**
 * The opening: a dark curtain that is already in place when the page paints,
 * then the logo mark, the wordmark and a line of light, then the curtain
 * lifts and the app is underneath, loaded. Roughly 1.6 seconds, and the
 * backend plays the chime at the same moment it says "yes, animate".
 *
 * Started into the tray, reloaded, or switched off in Settings, the backend
 * answers "no" and the curtain lifts at once — nothing flashes.
 */
const SHOW_MS = 1500;
const LIFT_MS = 500;

/** How long the whole thing takes, for anyone scheduling around it. */
export const SPLASH_TOTAL_MS = SHOW_MS + LIFT_MS;

type Phase = 'pending' | 'playing' | 'lifting' | 'done';

// The backend answers "yes" exactly once per process, so the question is
// asked exactly once per page too — however many times the effect runs.
let asked: Promise<{ animation: boolean; sound: boolean }> | null = null;
const greet = () => (asked ??= api.startupGreeting());

export function Splash() {
  const [phase, setPhase] = useState<Phase>('pending');
  // A replay fades the curtain in over the app; the first one is simply there.
  const [again, setAgain] = useState(false);
  // The saved language arrives a moment after the first paint; follow it.
  const [, setLang] = useState(0);
  useEffect(() => {
    const bump = () => setLang((n) => n + 1);
    window.addEventListener('language-changed', bump);
    return () => window.removeEventListener('language-changed', bump);
  }, []);

  useEffect(() => {
    let cancelled = false;
    const timers: number[] = [];
    const play = (replay = false) => {
      setAgain(replay);
      timers.forEach((id) => window.clearTimeout(id));
      timers.length = 0;
      setPhase('playing');
      timers.push(window.setTimeout(() => setPhase('lifting'), SHOW_MS));
      timers.push(window.setTimeout(() => setPhase('done'), SHOW_MS + LIFT_MS));
    };
    void greet()
      .then((greeting) => {
        if (cancelled) return;
        if (greeting?.animation) play();
        else setPhase('done');
      })
      // A backend that cannot answer must never leave a black curtain up.
      .catch(() => !cancelled && setPhase('done'));
    // Brought back from the tray, the shortcut or a second launch: the
    // backend says so, and the opening plays again over the loaded app.
    const unlisten = listen('opening', () => !cancelled && play(true));
    return () => {
      cancelled = true;
      timers.forEach((id) => window.clearTimeout(id));
      void unlisten.then((off) => off());
    };
  }, []);

  if (phase === 'done') return null;

  return (
    <div className={`splash splash-${phase}${again ? ' splash-again' : ''}`} aria-hidden="true" data-testid="splash">
      <div className="splash-glow" />
      <div className="splash-stage">
        <div className="splash-mark">
          <span>◆</span>
        </div>
        <div className="splash-word">GameHub</div>
        <div className="splash-line" />
        <div className="splash-tag">{t('splash.tagline')}</div>
      </div>
    </div>
  );
}
