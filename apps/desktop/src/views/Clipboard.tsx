import { useCallback, useEffect, useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { api, events, type ClipItem } from '../api';
import { formatRelativeDay } from '../format';
import { Confirm, Modal, PageHead, Toggle } from '../ui';

export function Clipboard({ enabled, onToast, onToggle }: { enabled: boolean; onToast: (title: string, body?: string) => void; onToggle: (enabled: boolean) => void }) {
  const [items, setItems] = useState<ClipItem[]>([]);
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState<ClipItem | null>(null);
  const [clearing, setClearing] = useState(false);

  const load = useCallback(() => void api.getClipboard(query).then(setItems), [query]);
  useEffect(() => {
    load();
    const pending = events.onClipboardUpdated(load);
    return () => {
      void pending.then((off) => off());
    };
  }, [load]);

  return (
    <div className="view">
      <PageHead title="Utklippstavle" blurb="De siste 100 tingene du har kopiert, bare på denne PC-en. Ting som ser ut som passord eller nøkler hoppes over.">
        <label className="row" style={{ gap: 8, fontSize: 12.5, color: 'var(--ink-dim)' }}>
          Historikk <Toggle checked={enabled} onChange={onToggle} label="Utklippshistorikk" />
        </label>
        <button className="btn btn-ghost btn-danger" onClick={() => setClearing(true)}>Tøm alt</button>
      </PageHead>
      <div className="filters">
        <div className="search-wrap" style={{ maxWidth: 420 }}>
          <span className="search-icon">⌕</span>
          <input className="search" placeholder="Søk i utklippstavlen …" value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
      </div>
      {items.length === 0 ? (
        <p className="empty">{enabled ? 'Ingenting kopiert ennå.' : 'Historikken er slått av, så ingenting registreres.'}</p>
      ) : (
        items.map((item, index) => (
          <div key={item.id} className="list-row" style={{ ['--i' as string]: Math.min(index, 15) }}>
            <span className="name" title={item.text}>
              {item.pinned && '📌 '}{item.isUrl && '🔗 '}
              {item.text.slice(0, 120).replace(/\s+/g, ' ')}{item.length > 120 && '…'}
            </span>
            <span className="meta">{item.length.toLocaleString('nb-NO')} tegn · {formatRelativeDay(item.copiedAt)}</span>
            <button className="btn sm btn-ghost" onClick={() => setOpen(item)}>Åpne</button>
            <button className="btn sm" onClick={async () => { await writeText(item.text); onToast('Kopiert igjen'); }}>Kopier</button>
            <button className="btn sm btn-ghost" onClick={async () => { await api.pinClip(item.id, !item.pinned); load(); }}>{item.pinned ? 'Løsne' : 'Fest'}</button>
          </div>
        ))
      )}
      {open && (
        <Modal
          title={`Kopiert ${formatRelativeDay(open.copiedAt)}`}
          onClose={() => setOpen(null)}
          actions={
            <>
              <button className="btn btn-danger" onClick={async () => { await api.deleteClip(open.id); setOpen(null); load(); }}>Slett</button>
              <button className="btn btn-accent" onClick={async () => { await writeText(open.text); setOpen(null); onToast('Kopiert igjen'); }}>Kopier igjen</button>
            </>
          }
        >
          <p className="version-note">{open.length.toLocaleString('nb-NO')} tegn{open.truncated && ' · lagret forkortet'}</p>
          <div className="notes" style={{ maxHeight: 320 }}>{open.text}</div>
        </Modal>
      )}
      {clearing && (
        <Confirm title="Slette hele utklippshistorikken?" confirmLabel="Tøm" danger onCancel={() => setClearing(false)} onConfirm={async () => { setClearing(false); await api.clearClipboard(); load(); onToast('Utklippshistorikken er tømt'); }} />
      )}
    </div>
  );
}
