import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { events, type Screenshot } from '../api';

/**
 * The screenshot you just took, shown in the app the moment it is saved — for
 * the hotkey and the button alike, since both emit the same event.
 */
export function ShotPreview({ onOpen }: { onOpen: (shot: Screenshot) => void }) {
  const [shot, setShot] = useState<Screenshot | null>(null);

  useEffect(() => {
    let timer: number | undefined;
    const pending = events.onScreenshotTaken((taken) => {
      if (!taken || typeof taken !== 'object' || !('path' in taken)) return;
      setShot(taken);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setShot(null), 7000);
    });
    return () => {
      window.clearTimeout(timer);
      void pending.then((off) => off());
    };
  }, []);

  if (!shot) return null;

  return (
    <div className="shot-preview" role="status" aria-live="polite">
      <button
        className="shot-preview-image"
        onClick={() => {
          onOpen(shot);
          setShot(null);
        }}
        aria-label="Åpne bildet i full størrelse"
      >
        <img src={convertFileSrc(shot.path)} alt="" />
      </button>
      <div className="shot-preview-text">
        <strong>Screenshot lagret</strong>
        <span>{shot.gameName}</span>
      </div>
      <button className="shot-preview-close" onClick={() => setShot(null)} aria-label="Lukk">
        ×
      </button>
    </div>
  );
}
