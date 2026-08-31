import { readManifest } from '@/lib/releases';
import { latestRelease } from '@/lib/version';
import { ReleaseCard } from '@/components/ReleaseCard';

export const dynamic = 'force-dynamic';

export const metadata = { title: 'Versjoner — GameHub' };

export default async function VersionsPage() {
  const manifest = await readManifest();
  const latest = latestRelease(manifest.releases);

  return (
    <div className="wrap" style={{ paddingTop: 54, paddingBottom: 40 }}>
      <h2 className="section">Versjoner</h2>
      <p className="section-note">Alt som har kommet, nyeste først.</p>

      {manifest.releases.length === 0 ? (
        <div className="panel">
          <p className="empty" style={{ padding: '24px 0' }}>
            Ingen versjoner er publisert ennå.
          </p>
        </div>
      ) : (
        manifest.releases.map((release) => (
          <ReleaseCard
            key={release.version}
            release={release}
            latest={release.version === latest?.version}
          />
        ))
      )}
    </div>
  );
}
