'use client';

import { useState } from 'react';
import { upload } from '@vercel/blob/client';
import type { Release, ReleaseFile } from '@/lib/types';
import { formatDate, formatSize } from '@/lib/format';
import { parseVersion } from '@/lib/version';

interface Props {
  initialReleases: Release[];
  username: string;
  defaultPassword: boolean;
  blobReady: boolean;
}

/** Anything over about 100 MB is uploaded in parts, which is faster and retries. */
const MULTIPART_OVER = 100 * 1024 * 1024;

export function AdminClient({ initialReleases, username, defaultPassword, blobReady }: Props) {
  const [releases, setReleases] = useState(initialReleases);
  const [version, setVersion] = useState('');
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [kind, setKind] = useState<ReleaseFile['kind']>('installer');
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState<string>('');
  const [percent, setPercent] = useState<number | null>(null);
  const [error, setError] = useState('');
  const [done, setDone] = useState('');

  const versionOk = parseVersion(version) !== null;
  const canPublish = versionOk && file !== null && !busy && blobReady;

  async function publish(event: React.FormEvent) {
    event.preventDefault();
    if (!file) return;

    setError('');
    setDone('');

    try {
      setBusy(`Laster opp ${file.name} …`);
      setPercent(0);
      // Straight from the browser to Blob. A Vercel function refuses a body
      // over about 4.5 MB, so routing the file through the server would fail
      // for any real installer.
      const blob = await upload(`releases/${version}/${file.name}`, file, {
        access: 'public',
        handleUploadUrl: '/api/upload',
        multipart: file.size > MULTIPART_OVER,
        onUploadProgress: (event) => setPercent(Math.round(event.percentage)),
      });

      setPercent(null);
      setBusy('Lagrer versjonen …');
      const response = await fetch('/api/releases', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          version,
          name,
          description,
          files: [{ name: file.name, url: blob.url, size: file.size, kind }],
        }),
      });

      const body = await response.json().catch(() => ({}));
      if (!response.ok) {
        setError(body.error ?? 'Kunne ikke lagre versjonen.');
        return;
      }

      setReleases(body.releases ?? []);
      setDone(`Versjon ${version} er publisert.`);
      setVersion('');
      setName('');
      setDescription('');
      setFile(null);
      // Clears the file input, which React cannot reset on its own.
      (document.getElementById('file') as HTMLInputElement | null)?.form?.reset();
    } catch (uploadError) {
      setError(
        uploadError instanceof Error ? uploadError.message : 'Opplastingen mislyktes.',
      );
    } finally {
      setBusy('');
      setPercent(null);
    }
  }

  async function remove(target: Release) {
    if (!window.confirm(`Slette versjon ${target.version} og filene som hører til?`)) return;
    setError('');
    setDone('');
    setBusy(`Sletter ${target.version} …`);
    try {
      const response = await fetch(`/api/releases?version=${encodeURIComponent(target.version)}`, {
        method: 'DELETE',
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok) {
        setError(body.error ?? 'Kunne ikke slette.');
        return;
      }
      setReleases(body.releases ?? []);
      setDone(`Versjon ${target.version} er slettet.`);
    } finally {
      setBusy('');
    }
  }

  return (
    <div className="wrap" style={{ paddingTop: 46, paddingBottom: 50 }}>
      <div className="row" style={{ marginBottom: 20 }}>
        <div>
          <h2 className="section" style={{ marginBottom: 2 }}>
            Admin
          </h2>
          <p className="section-note" style={{ margin: 0 }}>
            Logget inn som {username}.
          </p>
        </div>
        <button
          className="btn"
          style={{ marginLeft: 'auto' }}
          onClick={async () => {
            await fetch('/api/logout', { method: 'POST' });
            window.location.href = '/';
          }}
        >
          Logg ut
        </button>
      </div>

      {!blobReady && (
        <div className="notice notice-bad">
          <strong>Fillagringen er ikke satt opp</strong>
          Opplasting virker ikke før du har laget et Blob-lager i Vercel. Se SETUP.md — det tar et par
          minutter, og du trenger ikke endre koden.
        </div>
      )}

      {defaultPassword && (
        <div className="notice notice-warn">
          <strong>Passordet er fortsatt admin123</strong>
          Adressen /admin er lett å gjette, og den som kommer inn kan legge ut en hvilken som helst
          .exe som folk laster ned og kjører. Sett <code>ADMIN_PASSWORD</code> til noe annet under
          Settings → Environment Variables i Vercel før du deler siden med noen.
        </div>
      )}

      {error && <div className="notice notice-bad">{error}</div>}
      {done && <div className="notice notice-good">{done}</div>}

      <form className="panel" onSubmit={publish}>
        <h3 style={{ marginTop: 0, marginBottom: 18 }}>Ny versjon</h3>

        <div className="field">
          <label htmlFor="version">Versjonsnummer</label>
          <input
            id="version"
            type="text"
            placeholder="0.3.0"
            value={version}
            onChange={(e) => setVersion(e.target.value)}
            required
          />
          <p className="hint">
            {version && !versionOk
              ? 'Må være tre tall med punktum mellom, for eksempel 0.3.0.'
              : 'Samme nummer som i tauri.conf.json. Laster du opp til en versjon som finnes, legges filen til der.'}
          </p>
        </div>

        <div className="field">
          <label htmlFor="name">Navn på versjonen</label>
          <input
            id="name"
            type="text"
            placeholder="Screenshots og sikkerhetskopier"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <p className="hint">Valgfritt. Vises ved siden av nummeret.</p>
        </div>

        <div className="field">
          <label htmlFor="description">Hva er nytt</label>
          <textarea
            id="description"
            placeholder={'Screenshots vises nå i appen med én gang.\nHurtigtastene for søk og navigasjon virker.\nDataene dine blir tatt vare på når du oppdaterer.'}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
          <p className="hint">Vises på nedlastingssiden og under Versjoner. Linjeskift beholdes.</p>
        </div>

        <div className="field">
          <label htmlFor="kind">Type fil</label>
          <select id="kind" value={kind} onChange={(e) => setKind(e.target.value as ReleaseFile['kind'])}>
            <option value="installer">Installer — den store nedlastingsknappen peker hit</option>
            <option value="portable">Portabel — kjøres uten å installere</option>
            <option value="other">Annet</option>
          </select>
        </div>

        <div className="field">
          <label htmlFor="file">Fil</label>
          <input
            id="file"
            type="file"
            accept=".exe,.msi,.zip"
            onChange={(e) => setFile(e.target.files?.[0] ?? null)}
            required
          />
          <p className="hint">
            {file
              ? `${file.name} — ${formatSize(file.size)}`
              : '.exe, .msi eller .zip. Filen går rett fra nettleseren til lageret.'}
          </p>
        </div>

        <button className="btn btn-accent" type="submit" disabled={!canPublish}>
          {busy && <span className="spinner" aria-hidden="true" />}
          {busy || 'Publiser'}
        </button>
        {busy && (
          <>
            {percent !== null && (
              <div className="bar" style={{ marginTop: 14 }}>
                <div className="bar-fill" style={{ width: `${percent}%` }} />
              </div>
            )}
            <p className="hint" style={{ marginTop: 10 }}>
              {percent !== null
                ? `${percent}% lastet opp. Ikke lukk fanen.`
                : 'Ikke lukk fanen.'}
            </p>
          </>
        )}
      </form>

      <h3 style={{ marginBottom: 12 }}>Publisert</h3>
      {releases.length === 0 ? (
        <p className="empty">Ingenting er lagt ut ennå.</p>
      ) : (
        releases.map((release) => (
          <div className="admin-release" key={release.version}>
            <div className="what">
              <strong>{release.version}</strong> {release.name}
              <small>
                {formatDate(release.publishedAt)} · {release.files.length} fil
                {release.files.length === 1 ? '' : 'er'} ·{' '}
                {formatSize(release.files.reduce((sum, f) => sum + f.size, 0))}
              </small>
            </div>
            <button className="btn" onClick={() => void remove(release)} disabled={Boolean(busy)}>
              Slett
            </button>
          </div>
        ))
      )}
    </div>
  );
}
