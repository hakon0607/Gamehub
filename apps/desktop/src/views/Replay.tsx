import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, events, type Clip, type ReplayStatus, type Settings } from '../api';
import { formatBytes, formatDuration, formatRelativeDay, formatSeconds } from '../format';
import { Confirm, PageHead, Slider, Toggle, useBusy } from '../ui';
import { t, tr } from '../i18n';

/**
 * Instant replay: the buffer's state, the length of it, and the clips saved.
 *
 * The page is honest about the two things that stop it working — no ffmpeg,
 * and exclusive fullscreen — and shows whether the recording has sound, since
 * a silent clip is the bug this version exists to fix.
 */
export function Replay({
  settings,
  onSettings,
  onToast,
  onOpenSettings,
  hotkey,
}: {
  settings: Settings;
  onSettings: (next: Settings) => Promise<void>;
  onToast: (title: string, body?: string) => void;
  onOpenSettings: () => void;
  hotkey: string;
}) {
  const [status, setStatus] = useState<ReplayStatus | null>(null);
  const [clips, setClips] = useState<Clip[]>([]);
  const [watching, setWatching] = useState<Clip | null>(null);
  const [deleting, setDeleting] = useState<Clip | null>(null);
  const [game, setGame] = useState('all');
  // Trimming: a start and end inside the clip being watched.
  const [trim, setTrim] = useState<{ start: number; end: number } | null>(null);
  const [trimming, setTrimming] = useState(false);
  const videoRef = useRef<HTMLVideoElement>(null);
  const [playhead, setPlayhead] = useState(0);
  const { busy, run } = useBusy();

  const load = useCallback(() => {
    void api.replayStatus().then(setStatus);
    void api.getClips().then(setClips);
  }, []);

  useEffect(() => {
    load();
    const pending = [events.onClipSaved(load), events.onReplayState(load)];
    // The buffer fills by the second; the meter should move with it.
    const ticking = window.setInterval(() => void api.replayStatus().then(setStatus), 1000);
    return () => {
      window.clearInterval(ticking);
      for (const p of pending) void p.then((off) => off());
    };
  }, [load]);

  const replay = settings.replay;
  const setReplay = (patch: Partial<Settings['replay']>) => onSettings({ ...settings, replay: { ...replay, ...patch } });

  const save = (seconds?: number) =>
    run(async () => {
      try {
        const clip = await api.saveReplay(seconds);
        onToast(t('toast.clip_saved'), `${clip.gameName} · ${formatSeconds(clip.seconds)}${clip.hasAudio ? t('toast.clip_with_audio') : t('toast.clip_without_audio')}`);
        load();
      } catch (error) {
        onToast(t('toast.clip_failed'), tr(error));
      }
    });

  const toggle = async () => {
    try {
      await api.setReplayEnabled(!replay.enabled);
      await onSettings({ ...settings, replay: { ...replay, enabled: !replay.enabled } });
      load();
    } catch (error) {
      onToast(t('toast.replay_failed'), tr(error));
      load();
    }
  };

  const saveTrim = async (clip: Clip, range: { start: number; end: number }, replace: boolean) => {
    setTrimming(true);
    try {
      const made = await api.trimClip(clip.id, range.start, range.end, replace);
      onToast(t('replay.trim_done'), `${made.gameName} · ${formatSeconds(made.seconds)}`);
      setTrim(null);
      setWatching(made);
      load();
    } catch (error) {
      onToast(t('replay.trim_failed'), tr(error));
    } finally {
      setTrimming(false);
    }
  };

  const games = useMemo(() => [...new Set(clips.map((c) => c.gameName))], [clips]);
  const shown = clips.filter((c) => game === 'all' || c.gameName === game);
  const totalBytes = clips.reduce((sum, c) => sum + c.sizeBytes, 0);

  if (!status) return <p className="empty">{t('common.loading')}</p>;

  if (!status.ffmpegPath) {
    return (
      <div className="view">
        <PageHead title={t('replay.title')} />
        <div className="notice danger">{t('replay.no_ffmpeg')}</div>
      </div>
    );
  }

  const fill = status.bufferSeconds > 0 ? (status.bufferedSeconds / status.bufferSeconds) * 100 : 0;
  const lengths = [15, 30, 60, 120, 300, 600].filter((s) => s <= status.bufferSeconds);

  return (
    <div className="view">
      <PageHead
        title={t('replay.title')}
        blurb={t('replay.blurb', { key: hotkey })}
      >
        <button className="btn" onClick={onOpenSettings}>
          {t('replay.more_settings')}
        </button>
      </PageHead>

      <div className="rec-hero">
        <div className={`panel${status.running ? ' glow' : ''}`}>
          <div className="row" style={{ justifyContent: 'space-between' }}>
            <h3 className="section-title" style={{ margin: 0 }}>
              <span className={`pulse${status.running ? ' rec' : ''}`} style={status.running ? {} : { animation: 'none', background: 'var(--ink-faint)' }} />
              {status.running ? t('replay.recording') : replay.enabled ? t('replay.on_not_recording') : t('replay.off')}
            </h3>
            <label className="row" style={{ gap: 8, fontSize: 12.5, color: 'var(--ink-dim)' }}>
              {replay.enabled ? t('common.on') : t('common.off')} <Toggle checked={replay.enabled} onChange={() => void toggle()} label={t('replay.title')} />
            </label>
          </div>

          {status.problem && <div className="notice danger" style={{ marginTop: 12 }}>{tr(status.problem)}</div>}

          <div className="buffer-meter">
            <span style={{ width: `${status.running ? fill : 0}%` }} />
          </div>
          <p className="note">
            {status.running
              ? t('replay.buffer_status', { have: formatSeconds(status.bufferedSeconds), total: formatSeconds(status.bufferSeconds), mb: status.bufferEstimateMb })
              : t('replay.buffer_idle', { total: formatSeconds(status.bufferSeconds), mb: status.bufferEstimateMb })}
          </p>

          <div className="row" style={{ marginTop: 14 }}>
            <span className={`badge-pill ${status.running ? (status.audio.systemAudio ? 'ok' : 'warn') : ''}`}>
              {status.running
                ? status.audio.systemAudio
                  ? t('replay.audio_from', { device: tr(status.audio.systemDevice ?? '') })
                  : replay.systemAudio
                    ? t('replay.audio_none')
                    : t('replay.audio_off')
                : replay.systemAudio
                  ? t('replay.audio_on')
                  : t('replay.audio_off')}
            </span>
            {status.audio.microphone && <span className="badge-pill ok">{t('replay.mic')}</span>}
            {status.running && (
              <span className="badge-pill">
                {replay.scaleHeight ? `${replay.scaleHeight}p` : t('replay.native')} · {replay.fps} fps
              </span>
            )}
          </div>
          {status.audio.note && replay.systemAudio && (
            <p className="note" style={{ marginTop: 8, color: 'var(--warn)' }}>
              {tr(status.audio.note)}
            </p>
          )}

          <div style={{ marginTop: 18 }}>
            <div className="row" style={{ justifyContent: 'space-between', marginBottom: 6 }}>
              <span className="field-label">{t('replay.buffer_length')}</span>
              <span className="note" style={{ fontSize: 12 }}>
                {formatSeconds(status.minBufferSeconds)} – {formatSeconds(status.maxBufferSeconds)}
              </span>
            </div>
            <Slider
              value={replay.bufferSeconds}
              min={status.minBufferSeconds}
              max={status.maxBufferSeconds}
              step={15}
              format={formatSeconds}
              onCommit={(seconds) => {
                if (seconds !== replay.bufferSeconds) void setReplay({ bufferSeconds: seconds, saveSeconds: Math.min(replay.saveSeconds, seconds) });
              }}
            />
            <p className="note" style={{ fontSize: 12, marginTop: 4 }}>
              {t('replay.buffer_hint')}
            </p>
          </div>
        </div>

        <div className="panel">
          <h3 className="section-title" style={{ marginTop: 0 }}>
            {t('replay.save_now')}
          </h3>
          <p className="note" style={{ marginBottom: 12 }}>
            {t('replay.save_hint', { key: hotkey, time: formatSeconds(replay.saveSeconds) })}
          </p>
          <div className="quick-lengths">
            {lengths.map((seconds) => (
              <button
                key={seconds}
                className={`btn sm${seconds === replay.saveSeconds ? ' btn-accent' : ''}`}
                disabled={busy || !status.running}
                onClick={() => void save(seconds)}
              >
                {formatSeconds(seconds)}
              </button>
            ))}
          </div>
          <p className="note" style={{ fontSize: 12, marginTop: 10 }}>
            {t('replay.short_hint')}
          </p>
          <div className="row" style={{ marginTop: 14 }}>
            <button className="btn btn-accent" disabled={busy || !status.running} onClick={() => void save()}>
              {t('replay.save_button', { time: formatSeconds(replay.saveSeconds) })}
            </button>
            <button className="btn" onClick={() => void revealItemInDir(status.clipFolder)}>
              {t('common.open_folder')}
            </button>
          </div>
        </div>
      </div>

      <div className="page-head" style={{ marginBottom: 12 }}>
        <div>
          <h2 className="section-title" style={{ margin: 0 }}>
            {t('replay.clips')} <span className="countdown">{t('replay.clips_meta', { n: clips.length, size: formatBytes(totalBytes) })}</span>
          </h2>
        </div>
      </div>
      {games.length > 1 && (
        <div className="filters">
          <button className="chip" aria-pressed={game === 'all'} onClick={() => setGame('all')}>
            {t('common.all')}
          </button>
          {games.map((name) => (
            <button key={name} className="chip" aria-pressed={game === name} onClick={() => setGame(name)}>
              {name}
            </button>
          ))}
        </div>
      )}

      {clips.length === 0 ? (
        <p className="empty">
          {t('replay.empty', { key: hotkey, folder: status.clipFolder })}
        </p>
      ) : (
        <div className="shots">
          {shown.map((clip, index) => (
            <figure key={clip.id} className="shot" style={{ ['--i' as string]: index }}>
              <button onClick={() => setWatching(clip)} aria-label={t('replay.play_clip', { name: clip.gameName })}>
                <video src={convertFileSrc(clip.path)} preload="metadata" muted />
                <span className="play-glyph">▶</span>
                <span className="duration">{formatDuration(clip.seconds)}</span>
                {!clip.hasAudio && <span className="muted-mark">🔇</span>}
              </button>
              <figcaption>
                <span className="name" title={clip.gameName}>
                  {clip.gameName}
                </span>
                <span>{formatRelativeDay(clip.recordedAt)}</span>
                <button
                  className="shot-star"
                  aria-label={clip.favorite ? t('replay.unfavorite') : t('replay.favorite')}
                  onClick={async () => {
                    await api.setClipFavorite(clip.id, !clip.favorite);
                    load();
                  }}
                >
                  {clip.favorite ? '★' : '☆'}
                </button>
              </figcaption>
            </figure>
          ))}
        </div>
      )}

      {watching && (
        <div className="scrim" role="dialog" aria-modal="true" onClick={() => { setWatching(null); setTrim(null); }}>
          <div className="viewer" onClick={(e) => e.stopPropagation()}>
            <video
              ref={videoRef}
              src={convertFileSrc(watching.path)}
              controls
              autoPlay
              style={{ width: 'min(1100px, 92vw)' }}
              onTimeUpdate={(e) => {
                const v = e.currentTarget;
                setPlayhead(v.currentTime);
                // While trimming, playback stops at the chosen end so the cut can be judged.
                if (trim && v.currentTime >= trim.end) v.pause();
              }}
            />
            {trim && (
              <div className="trim">
                <div
                  className="trim-bar"
                  onClick={(e) => {
                    const rect = e.currentTarget.getBoundingClientRect();
                    const at = ((e.clientX - rect.left) / rect.width) * (videoRef.current?.duration || watching.seconds);
                    if (videoRef.current) videoRef.current.currentTime = at;
                  }}
                >
                  <span
                    className="trim-range"
                    style={{
                      left: `${(trim.start / (videoRef.current?.duration || watching.seconds)) * 100}%`,
                      width: `${((trim.end - trim.start) / (videoRef.current?.duration || watching.seconds)) * 100}%`,
                    }}
                  />
                  <span className="trim-head" style={{ left: `${(playhead / (videoRef.current?.duration || watching.seconds)) * 100}%` }} />
                </div>
                <div className="row" style={{ marginTop: 10, gap: 8, flexWrap: 'wrap' }}>
                  <button className="btn sm" onClick={() => setTrim({ start: Math.min(playhead, trim.end - 1), end: trim.end })}>
                    {t('replay.trim_set_start')}
                  </button>
                  <button className="btn sm" onClick={() => setTrim({ start: trim.start, end: Math.max(playhead, trim.start + 1) })}>
                    {t('replay.trim_set_end')}
                  </button>
                  <button
                    className="btn sm btn-ghost"
                    onClick={() => {
                      if (videoRef.current) {
                        videoRef.current.currentTime = trim.start;
                        void videoRef.current.play();
                      }
                    }}
                  >
                    {t('replay.trim_preview')}
                  </button>
                  <span className="note" style={{ alignSelf: 'center' }}>
                    {t('replay.trim_range', { start: formatSeconds(Math.round(trim.start)), end: formatSeconds(Math.round(trim.end)), length: formatSeconds(Math.round(trim.end - trim.start)) })}
                  </span>
                </div>
              </div>
            )}
            <div className="dialog-actions">
              <span className="note" style={{ flex: 1, alignSelf: 'center' }}>
                {watching.gameName} · {formatBytes(watching.sizeBytes)}
                {watching.hasAudio ? '' : t('replay.without_audio')}
              </span>
              {trim ? (
                <>
                  <button className="btn" disabled={trimming} onClick={() => setTrim(null)}>
                    {t('common.cancel')}
                  </button>
                  <button
                    className="btn"
                    disabled={trimming}
                    title={t('replay.trim_replace_hint')}
                    onClick={() => void saveTrim(watching, trim, true)}
                  >
                    {t('replay.trim_replace')}
                  </button>
                  <button className="btn btn-accent" disabled={trimming} onClick={() => void saveTrim(watching, trim, false)}>
                    {trimming ? t('replay.trim_working') : t('replay.trim_save')}
                  </button>
                </>
              ) : (
                <>
                  <button
                    className="btn"
                    onClick={() => {
                      const total = videoRef.current?.duration || watching.seconds;
                      setTrim({ start: 0, end: total });
                    }}
                  >
                    {t('replay.trim')}
                  </button>
                  <button className="btn" onClick={() => void revealItemInDir(watching.path)}>
                    {t('common.show_in_folder')}
                  </button>
                  <button className="btn btn-danger" onClick={() => setDeleting(watching)}>
                    {t('common.delete')}
                  </button>
                  <button className="btn btn-accent" onClick={() => setWatching(null)}>
                    {t('common.close')}
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      )}

      {deleting && (
        <Confirm
          title={t('replay.delete_confirm')}
          body={t('replay.delete_body')}
          confirmLabel={t('common.delete')}
          danger
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            try {
              await api.deleteReplayClip(deleting.id);
            } catch (error) {
              onToast(t('replay.delete_failed'), tr(error));
            }
            setDeleting(null);
            setWatching(null);
            load();
          }}
        />
      )}
    </div>
  );
}
