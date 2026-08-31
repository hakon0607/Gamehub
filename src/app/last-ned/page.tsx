import Link from 'next/link';
import { readManifest } from '@/lib/releases';
import { latestRelease, primaryFile } from '@/lib/version';
import { formatSize } from '@/lib/format';
import { ReleaseCard } from '@/components/ReleaseCard';

export const dynamic = 'force-dynamic';

export const metadata = { title: 'Last ned GameHub' };

export default async function DownloadPage() {
  const manifest = await readManifest();
  const latest = latestRelease(manifest.releases);
  const file = primaryFile(latest);

  return (
    <div className="wrap" style={{ paddingTop: 54, paddingBottom: 40 }}>
      <h2 className="section">Last ned</h2>
      <p className="section-note">Windows 10 og 11, 64-bit. Gratis.</p>

      {!latest || !file ? (
        <div className="panel">
          <p className="empty" style={{ padding: '24px 0' }}>
            Det ligger ingen versjon her ennå. Kom tilbake om litt.
          </p>
        </div>
      ) : (
        <>
          <div className="panel" style={{ textAlign: 'center', padding: '38px 26px' }}>
            <a className="btn btn-accent" href={file.url} download style={{ fontSize: 16 }}>
              Last ned GameHub {latest.version}
              <small style={{ opacity: 0.8, fontWeight: 400 }}>{formatSize(file.size)}</small>
            </a>
            <p className="sub" style={{ marginTop: 16, marginBottom: 0 }}>
              {file.name}
            </p>
          </div>

          <div className="notice notice-warn">
            <strong>Windows sier kanskje at utgiveren er ukjent</strong>
            Det er fordi filen ikke er kodesignert — et sertifikat koster penger i året. Velg{' '}
            <em>Mer informasjon</em> og så <em>Kjør likevel</em>.
          </div>

          <h2 className="section" style={{ marginTop: 34 }}>
            Denne versjonen
          </h2>
          <ReleaseCard release={latest} latest />

          <p style={{ marginTop: 24 }}>
            <Link href="/versjoner">Se alle versjoner og eldre nedlastinger →</Link>
          </p>
        </>
      )}
    </div>
  );
}
