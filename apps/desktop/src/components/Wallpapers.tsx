import { useCallback, useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, type WallpaperPicture, type WallpaperStatus, type WallpaperTarget } from '../api';
import { formatBytes } from '../format';
import { t, tr } from '../i18n';

/**
 * Wallpapers: the desktop background and the lock screen picture.
 *
 * Two cards, one per place. Each shows the picture GameHub chose, its size,
 * and whether Windows is showing it now — which Windows can only tell us for
 * the desktop; the lock screen has no way to be read back, so that card says
 * what was last sent. Every button talks to Windows through the Rust side;
 * nothing here is a mockup.
 */
export function Wallpapers({ onToast }: { onToast: (title: string, body?: string) => void }) {
  const [status, setStatus] = useState<WallpaperStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const load = useCallback(() => {
    void api.wallpaperStatus().then(setStatus).catch((error) => onToast(t('wp.status_failed'), tr(error)));
  }, [onToast]);

  useEffect(load, [load]);

  const run = async (key: string, work: () => Promise<WallpaperStatus>, done: string) => {
    setBusy(key);
    try {
      setStatus(await work());
      onToast(done);
    } catch (error) {
      onToast(t('wp.set_failed'), tr(error));
    } finally {
      setBusy(null);
    }
  };

  const pick = async (): Promise<string | null> => {
    const picked = await open({
      multiple: false,
      title: t('wp.pick'),
      filters: [{ name: t('game.pick_image'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp'] }],
    });
    return typeof picked === 'string' ? picked : null;
  };

  const change = async (target: WallpaperTarget, monitor?: string) => {
    const path = await pick();
    if (!path) return;
    await run(`${target}:${monitor ?? ''}`, () => api.setWallpaper(target, path, monitor), t('wp.set_ok'));
  };

  if (!status) return <p className="empty">{t('common.loading')}</p>;

  if (!status.supported) {
    return <div className="notice danger">{t('wp.unsupported')}</div>;
  }

  const card = (target: WallpaperTarget, picture: WallpaperPicture | null, active: 'yes' | 'no' | 'sent' | 'none') => {
    const icon = target === 'lock' ? '🔒' : '🏠';
    const badge =
      active === 'yes'
        ? { cls: 'ok', text: t('wp.active') }
        : active === 'sent'
          ? { cls: 'accent', text: t('wp.sent') }
          : active === 'no'
            ? { cls: 'warn', text: t('wp.not_active') }
            : { cls: '', text: t('wp.none_badge') };
    return (
      <div className={`wp-card${picture ? ' has-picture' : ''}`} data-setting={target === "lock" ? "wallpaper-lock" : undefined}>
        <div className="wp-head">
          <strong>
            <span aria-hidden="true">{icon}</span> {target === 'lock' ? t('wp.lock') : t('wp.desktop')}
          </strong>
          <span className={`badge-pill ${badge.cls}`}>{badge.text}</span>
        </div>
        <button className="wp-preview" onClick={() => void change(target)} aria-label={t('wp.change')} disabled={busy !== null}>
          {picture ? (
            <img src={convertFileSrc(picture.path)} alt="" />
          ) : (
            <span className="wp-empty">
              <span className="wp-empty-glyph">{icon}</span>
              {t('wp.none')}
            </span>
          )}
        </button>
        <div className="wp-meta">
          {picture ? (
            <>
              <span className="name" title={picture.path}>
                {picture.fileName}
              </span>
              <span className="note">{t('wp.size', { w: picture.width, h: picture.height, size: formatBytes(picture.bytes) })}</span>
            </>
          ) : (
            <span className="note">{t('wp.formats')}</span>
          )}
        </div>
        <div className="row wp-actions">
          <button className="btn sm btn-accent" disabled={busy !== null} onClick={() => void change(target)}>
            {t('wp.change')}
          </button>
          {picture && (
            <button
              className="btn sm"
              disabled={busy !== null}
              title={t('wp.remove_hint')}
              onClick={() => void run(`forget:${target}`, () => api.forgetWallpaper(target), t('wp.removed'))}
            >
              {t('wp.remove')}
            </button>
          )}
          <button
            className="btn sm btn-ghost"
            disabled={busy !== null}
            title={t('wp.default_hint')}
            onClick={() => void run(`default:${target}`, () => api.defaultWallpaper(target), t('wp.default_ok'))}
          >
            {t('wp.default')}
          </button>
        </div>
      </div>
    );
  };

  const desktopState = status.desktop ? (status.desktopActive ? 'yes' : 'no') : 'none';
  const lockState = status.lock ? 'sent' : 'none';

  return (
    <div className="wp">
      <p className="note" style={{ marginBottom: 14 }}>
        {t('wp.blurb')}
      </p>
      <div className="wp-grid">
        {card('lock', status.lock, lockState)}
        {card('desktop', status.desktop, desktopState)}
      </div>

      <div className="row" style={{ marginTop: 14, gap: 10, flexWrap: 'wrap' }}>
        <button
          className="btn"
          disabled={busy !== null || !status.desktop}
          title={t('wp.same_hint')}
          onClick={() => status.desktop && void run('same', () => api.setWallpaper('lock', status.desktop!.path), t('wp.set_ok'))}
        >
          {t('wp.same')}
        </button>
        <button
          className="btn btn-ghost"
          disabled={busy !== null || !status.lock}
          title={t('wp.same_hint_reverse')}
          onClick={() => status.lock && void run('same-reverse', () => api.setWallpaper('desktop', status.lock!.path), t('wp.set_ok'))}
        >
          {t('wp.same_reverse')}
        </button>
        <button className="btn btn-ghost" onClick={() => void revealItemInDir(status.folder)}>
          {t('common.open_folder')}
        </button>
      </div>

      {status.monitors.length > 1 && (
        <div className="wp-monitors">
          <div className="setting-group-title" style={{ padding: '18px 0 8px' }}>
            {t('wp.monitors')}
          </div>
          <p className="note" style={{ marginBottom: 10 }}>
            {t('wp.monitors_hint')}
          </p>
          {status.monitors.map((monitor) => {
            const showing = monitor.current.split(/[\\/]/).pop() ?? '';
            return (
              <div key={monitor.id} className="wp-monitor" data-setting={`wallpaper-monitor-${monitor.index}`}>
                <div className="wp-monitor-thumb">
                  {monitor.chosen ? <img src={convertFileSrc(monitor.chosen.path)} alt="" /> : <span>🖥</span>}
                </div>
                <div style={{ minWidth: 0, flex: 1 }}>
                  <strong>{t('wp.monitor', { n: monitor.index + 1 })}</strong>
                  <div className="note" title={monitor.current}>
                    {t('wp.monitor_hint', { w: monitor.width, h: monitor.height, file: showing || '—' })}
                  </div>
                  <div className="note" style={{ fontSize: 12 }}>
                    {monitor.chosen ? monitor.chosen.fileName : t('wp.monitor_all')}
                  </div>
                </div>
                <div className="row">
                  <button className="btn sm" disabled={busy !== null} onClick={() => void change('desktop', monitor.id)}>
                    {t('wp.change')}
                  </button>
                  {monitor.chosen && (
                    <button
                      className="btn sm btn-ghost"
                      disabled={busy !== null}
                      onClick={() => void run(`monitor-reset:${monitor.id}`, () => api.forgetWallpaper('desktop', monitor.id), t('wp.removed'))}
                    >
                      {t('wp.monitor_reset')}
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      <div className="notice info" style={{ marginTop: 16 }}>
        {t('wp.lock_note')}
      </div>
    </div>
  );
}
