import { useCallback, useEffect, useMemo, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { api, events, type Clip, type ReplayStatus, type Settings } from '../api';
import { formatBytes, formatDuration, formatRelativeDay, formatSeconds } from '../format';
import { Confirm, PageHead, Slider, Toggle, useBusy } from '../ui';

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
}: {
  settings: Settings;
  onSettings: (next: Settings) => Promise<void>;
  onToast: (title: string, body?: string) => void;
  onOpenSettings: () => void;
}) {
  const [status, setStatus] = useState<ReplayStatus | null>(null);
  const [clips, setClips] = useState<Clip[]>([]);
  const [watching, setWatching] = useState<Clip | null>(null);
  const [deleting, setDeleting] = useState<Clip | null>(null);
  const [game, setGame] = useState('all');
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
        onToast('Klipp lagret', `${clip.gameName} · ${formatSeconds(clip.seconds)}${clip.hasAudio ? ' · med lyd' : ' · uten lyd'}`);
        load();
      } catch (error) {
        onToast('Klippet ble ikke lagret', String(error));
      }
    });

  const toggle = async () => {
    try {
      await api.setReplayEnabled(!replay.enabled);
      await onSettings({ ...settings, replay: { ...replay, enabled: !replay.enabled } });
      load();
    } catch (error) {
      onToast('Replay startet ikke', String(error));
      load();
    }
  };

  const games = useMemo(() => [...new Set(clips.map((c) => c.gameName))], [clips]);
  const shown = clips.filter((c) => game === 'all' || c.gameName === game);
  const totalBytes = clips.reduce((sum, c) => sum + c.sizeBytes, 0);

  if (!status) return <p className="empty">Laster …</p>;

  if (!status.ffmpegPath) {
    return (
      <div className="view">
        <PageHead title="Replay" />
        <div className="notice danger">
          Replay trenger <strong>ffmpeg</strong>, og GameHub finner den ikke. Den følger normalt med installeren fra
          GitHub. Bygger du selv: kjør <code>pnpm fetch-ffmpeg</code> før du bygger, eller installer ffmpeg slik at den
          ligger i PATH. Resten av GameHub fungerer som normalt.
        </div>
      </div>
    );
  }

  const fill = status.bufferSeconds > 0 ? (status.bufferedSeconds / status.bufferSeconds) * 100 : 0;
  const lengths = [15, 30, 60, 120, 300, 600].filter((s) => s <= status.bufferSeconds);

  return (
    <div className="view">
      <PageHead
        title="Replay"
        blurb="Skjermen tas opp fortløpende, men bare de siste minuttene finnes — alt eldre overskrives og havner aldri på disken. F8 lagrer det du nettopp så."
      >
        <button className="btn" onClick={onOpenSettings}>
          Flere innstillinger
        </button>
      </PageHead>

      <div className="rec-hero">
        <div className={`panel${status.running ? ' glow' : ''}`}>
          <div className="row" style={{ justifyContent: 'space-between' }}>
            <h3 className="section-title" style={{ margin: 0 }}>
              <span className={`pulse${status.running ? ' rec' : ''}`} style={status.running ? {} : { animation: 'none', background: 'var(--ink-faint)' }} />
              {status.running ? 'Tar opp' : replay.enabled ? 'Slått på, men tar ikke opp' : 'Av'}
            </h3>
            <label className="row" style={{ gap: 8, fontSize: 12.5, color: 'var(--ink-dim)' }}>
              {replay.enabled ? 'På' : 'Av'} <Toggle checked={replay.enabled} onChange={() => void toggle()} label="Replay" />
            </label>
          </div>

          {status.problem && <div className="notice danger" style={{ marginTop: 12 }}>{status.problem}</div>}

          <div className="buffer-meter">
            <span style={{ width: `${status.running ? fill : 0}%` }} />
          </div>
          <p className="note">
            {status.running
              ? `${formatSeconds(status.bufferedSeconds)} av ${formatSeconds(status.bufferSeconds)} i bufferet · ca. ${status.bufferEstimateMb} MB på disk`
              : `Bufferet holder ${formatSeconds(status.bufferSeconds)} når det er på · ca. ${status.bufferEstimateMb} MB på disk`}
          </p>

          <div className="row" style={{ marginTop: 14 }}>
            <span className={`badge-pill ${status.running ? (status.audio.systemAudio ? 'ok' : 'warn') : ''}`}>
              {status.running
                ? status.audio.systemAudio
                  ? `🔊 Spillyd fra ${status.audio.systemDevice ?? 'standard lydenhet'}`
                  : replay.systemAudio
                    ? '🔇 Ingen spillyd'
                    : '🔇 Spillyd av'
                : replay.systemAudio
                  ? '🔊 Spillyd på'
                  : '🔇 Spillyd av'}
            </span>
            {status.audio.microphone && <span className="badge-pill ok">🎙 Mikrofon</span>}
            {status.running && (
              <span className="badge-pill">
                {replay.scaleHeight ? `${replay.scaleHeight}p` : 'Skjermens oppløsning'} · {replay.fps} fps
              </span>
            )}
          </div>
          {status.audio.note && replay.systemAudio && (
            <p className="note" style={{ marginTop: 8, color: 'var(--warn)' }}>
              {status.audio.note}
            </p>
          )}

          <div style={{ marginTop: 18 }}>
            <div className="row" style={{ justifyContent: 'space-between', marginBottom: 6 }}>
              <span className="field-label">Hvor mye som huskes</span>
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
              Lengre buffer koster bare diskplass, ikke ytelse. Endringen tar effekt med en gang.
            </p>
          </div>
        </div>

        <div className="panel">
          <h3 className="section-title" style={{ marginTop: 0 }}>
            Lagre nå
          </h3>
          <p className="note" style={{ marginBottom: 12 }}>
            <kbd className="key">F8</kbd> lagrer de siste {formatSeconds(replay.saveSeconds)}. Velg en annen lengde her:
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
            Har det gått kortere tid siden replay ble slått på, blir klippet bare så langt som det finnes opptak.
          </p>
          <div className="row" style={{ marginTop: 14 }}>
            <button className="btn btn-accent" disabled={busy || !status.running} onClick={() => void save()}>
              ⏺ Lagre {formatSeconds(replay.saveSeconds)}
            </button>
            <button className="btn" onClick={() => void revealItemInDir(status.clipFolder)}>
              Åpne mappen
            </button>
          </div>
        </div>
      </div>

      <div className="page-head" style={{ marginBottom: 12 }}>
        <div>
          <h2 className="section-title" style={{ margin: 0 }}>
            Klipp <span className="countdown">{clips.length} · {formatBytes(totalBytes)}</span>
          </h2>
        </div>
      </div>
      {games.length > 1 && (
        <div className="filters">
          <button className="chip" aria-pressed={game === 'all'} onClick={() => setGame('all')}>
            Alle
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
          Ingen klipp ennå. Slå på replay, spill litt, og trykk F8 når noe skjer — klippet havner her og under{' '}
          <code>{status.clipFolder}</code>.
        </p>
      ) : (
        <div className="shots">
          {shown.map((clip, index) => (
            <figure key={clip.id} className="shot" style={{ ['--i' as string]: index }}>
              <button onClick={() => setWatching(clip)} aria-label={`Spill av ${clip.gameName}`}>
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
                  aria-label={clip.favorite ? 'Fjern favoritt' : 'Marker som favoritt'}
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
        <div className="scrim" role="dialog" aria-modal="true" onClick={() => setWatching(null)}>
          <div className="viewer" onClick={(e) => e.stopPropagation()}>
            <video src={convertFileSrc(watching.path)} controls autoPlay style={{ width: 'min(1100px, 92vw)' }} />
            <div className="dialog-actions">
              <span className="note" style={{ flex: 1, alignSelf: 'center' }}>
                {watching.gameName} · {formatBytes(watching.sizeBytes)}
                {watching.hasAudio ? '' : ' · uten lyd'}
              </span>
              <button className="btn" onClick={() => void revealItemInDir(watching.path)}>
                Vis i mappe
              </button>
              <button className="btn btn-danger" onClick={() => setDeleting(watching)}>
                Slett
              </button>
              <button className="btn btn-accent" onClick={() => setWatching(null)}>
                Lukk
              </button>
            </div>
          </div>
        </div>
      )}

      {deleting && (
        <Confirm
          title="Slette dette klippet?"
          body="Filen slettes fra disken."
          confirmLabel="Slett"
          danger
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            try {
              await api.deleteReplayClip(deleting.id);
            } catch (error) {
              onToast('Klippet ble ikke slettet', String(error));
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
