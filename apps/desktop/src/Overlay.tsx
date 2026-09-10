import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { setLanguage, tr } from './i18n';

/**
 * The popup over the game: a card in the bottom-right corner that shows what
 * a shortcut just did while GameHub itself was hidden. The Rust side decides
 * when to show it and sends the words as message codes; this translates them
 * in the language it is told, shows for a few seconds and hides the window
 * again when the last card is gone.
 */
interface Popup {
  title: string;
  body: string;
  kind: 'ok' | 'error' | 'frozen' | 'shot' | 'clip' | 'replay';
  language: string;
  theme: string;
  seconds: number;
}

interface Card extends Popup {
  id: number;
  leaving: boolean;
}

const ICONS: Record<Popup['kind'], string> = { ok: '✓', error: '!', frozen: '❄', shot: '⎙', clip: '⏺', replay: '●' };

export function Overlay() {
  const [cards, setCards] = useState<Card[]>([]);
  const next = useRef(1);
  const timers = useRef<number[]>([]);

  useEffect(() => {
    document.documentElement.classList.add('overlay-window');
    const off = listen<Popup>('overlay-toast', (event) => {
      const popup = event.payload;
      setLanguage(popup.language);
      document.documentElement.dataset.theme = popup.theme || 'nattbla';
      const id = next.current++;
      // Newest at the bottom, at most three on screen.
      setCards((list) => [...list, { ...popup, id, leaving: false }].slice(-3));
      const ms = Math.max(1000, popup.seconds * 1000);
      timers.current.push(
        window.setTimeout(() => setCards((list) => list.map((c) => (c.id === id ? { ...c, leaving: true } : c))), ms - 260),
        window.setTimeout(() => setCards((list) => list.filter((c) => c.id !== id)), ms),
      );
    });
    return () => {
      void off.then((fn) => fn());
      for (const t of timers.current) window.clearTimeout(t);
    };
  }, []);

  // The window hides itself once nothing is left to show, so it never sits
  // invisibly in front of the game.
  useEffect(() => {
    if (cards.length === 0) void getCurrentWebviewWindow().hide();
  }, [cards.length]);

  return (
    <div className="overlay-stack">
      {cards.map((card) => (
        <div key={card.id} className={`overlay-card ${card.kind}${card.leaving ? ' leaving' : ''}`} role="status">
          <span className="overlay-icon">{ICONS[card.kind]}</span>
          <div className="overlay-text">
            <strong>{tr(card.title)}</strong>
            {card.body && <span>{tr(card.body)}</span>}
          </div>
        </div>
      ))}
    </div>
  );
}
