import { useCallback, useEffect, useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { api, events, type ClipItem } from '../api';
import { formatRelativeDay } from '../format';
import { Confirm, Modal, PageHead, Toggle } from '../ui';
import { formatNumber } from '../format';
import { t } from '../i18n';

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
      <PageHead title={t('clip.title')} blurb={t('clip.blurb')}>
        <label className="row" style={{ gap: 8, fontSize: 12.5, color: 'var(--ink-dim)' }}>
          {t('clip.history')} <Toggle checked={enabled} onChange={onToggle} label={t('clip.history')} />
        </label>
        <button className="btn btn-ghost btn-danger" onClick={() => setClearing(true)}>{t('clip.clear')}</button>
      </PageHead>
      <div className="filters">
        <div className="search-wrap" style={{ maxWidth: 420 }}>
          <span className="search-icon">⌕</span>
          <input className="search" placeholder={t('clip.search')} value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
      </div>
      {items.length === 0 ? (
        <p className="empty">{enabled ? t('clip.empty_on') : t('clip.empty_off')}</p>
      ) : (
        items.map((item, index) => (
          <div key={item.id} className="list-row" style={{ ['--i' as string]: Math.min(index, 15) }}>
            <span className="name" title={item.text}>
              {item.pinned && '📌 '}{item.isUrl && '🔗 '}
              {item.text.slice(0, 120).replace(/\s+/g, ' ')}{item.length > 120 && '…'}
            </span>
            <span className="meta">{t('clip.chars', { n: formatNumber(item.length) })} · {formatRelativeDay(item.copiedAt)}</span>
            <button className="btn sm btn-ghost" onClick={() => setOpen(item)}>{t('common.open')}</button>
            <button className="btn sm" onClick={async () => { await writeText(item.text); onToast(t('clip.copied')); }}>{t('clip.copy')}</button>
            <button className="btn sm btn-ghost" onClick={async () => { await api.pinClip(item.id, !item.pinned); load(); }}>{item.pinned ? t('clip.unpin') : t('clip.pin')}</button>
          </div>
        ))
      )}
      {open && (
        <Modal
          title={t('clip.copied_when', { when: formatRelativeDay(open.copiedAt) })}
          onClose={() => setOpen(null)}
          actions={
            <>
              <button className="btn btn-danger" onClick={async () => { await api.deleteClip(open.id); setOpen(null); load(); }}>{t('common.delete')}</button>
              <button className="btn btn-accent" onClick={async () => { await writeText(open.text); setOpen(null); onToast(t('clip.copied')); }}>{t('clip.copy_again')}</button>
            </>
          }
        >
          <p className="version-note">{t('clip.chars', { n: formatNumber(open.length) })}{open.truncated && t('clip.truncated')}</p>
          <div className="notes" style={{ maxHeight: 320 }}>{open.text}</div>
        </Modal>
      )}
      {clearing && (
        <Confirm title={t('clip.clear_confirm')} confirmLabel={t('clip.clear')} danger onCancel={() => setClearing(false)} onConfirm={async () => { setClearing(false); await api.clearClipboard(); load(); onToast(t('clip.cleared')); }} />
      )}
    </div>
  );
}
