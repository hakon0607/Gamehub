import Link from 'next/link';
import { readManifest } from '@/lib/releases';
import { latestRelease, primaryFile } from '@/lib/version';
import { formatSize } from '@/lib/format';

// Always rendered fresh, so a release published a minute ago is on the page.
export const dynamic = 'force-dynamic';

const FEATURES = [
  {
    icon: '◆',
    title: 'Ett bibliotek',
    body: 'Steam, Epic, Xbox, EA, Ubisoft Connect, Battle.net, GOG og Riot samlet ett sted. Hvert spill startes gjennom sin egen launcher, som normalt.',
  },
  {
    icon: '⟳',
    title: 'Finner spill selv',
    body: 'GameHub oppdager nyinstallerte spill av seg selv og legger dem til. Du kan også legge til spill manuelt, eller skjule dem du ikke vil se.',
  },
  {
    icon: '▤',
    title: 'Coverbilder',
    body: 'Henter coverbilder automatisk. Passer ikke bildet, laster du opp ditt eget og beskjærer det slik du vil ha det.',
  },
  {
    icon: '◷',
    title: 'Spilletid og rekker',
    body: 'Ser hvor lenge du har spilt hva, og hvor mange dager på rad. Kan slås helt av — da lagres ingenting.',
  },
  {
    icon: '⎙',
    title: 'Screenshots',
    body: 'Trykk hurtigtasten mens du spiller. Bildet havner under riktig spill og dukker opp i appen med én gang.',
  },
  {
    icon: '↺',
    title: 'Replay',
    body: 'Holder de siste sekundene i minnet, så du kan lagre noe som allerede har skjedd. Av som standard.',
  },
];

export default async function HomePage() {
  const manifest = await readManifest();
  const latest = latestRelease(manifest.releases);
  const file = primaryFile(latest);

  return (
    <div className="wrap">
      <section className="hero">
        <h1>Alle PC-spillene dine på ett sted</h1>
        <p className="lead">
          GameHub samler spillene fra alle launcherne dine i ett bibliotek, finner nye av seg selv, og
          starter hvert spill gjennom launcheren det hører hjemme i. Gratis, for Windows.
        </p>

        <div className="cta-row">
          {file ? (
            <a className="btn btn-accent" href={file.url} download>
              Last ned GameHub {latest?.version}
              <small style={{ opacity: 0.8, fontWeight: 400 }}>{formatSize(file.size)}</small>
            </a>
          ) : (
            <Link className="btn" href="/last-ned">
              Nedlasting kommer snart
            </Link>
          )}
          <Link className="btn" href="/versjoner">
            Se hva som er nytt
          </Link>
        </div>

        <p className="sub">
          Windows 10 og 11 · 64-bit{latest ? ` · versjon ${latest.version}` : ''}
        </p>
      </section>

      <div className="grid">
        {FEATURES.map((feature) => (
          <div className="card" key={feature.title}>
            <div className="icon" aria-hidden="true">
              {feature.icon}
            </div>
            <h3>{feature.title}</h3>
            <p>{feature.body}</p>
          </div>
        ))}
      </div>

      <section style={{ marginBottom: 40 }}>
        <h2 className="section">Alt ligger på din egen PC</h2>
        <p className="section-note" style={{ maxWidth: 640 }}>
          GameHub trenger ingen konto og ingen internettforbindelse for å virke. Biblioteket,
          coverbildene, screenshotene og spilletiden ligger i din egen mappe på maskinen — ingenting
          lastes opp noe sted. Appen tar en sikkerhetskopi automatisk før en ny versjon kjører første
          gang, så en oppdatering aldri koster deg det du har bygget opp.
        </p>
      </section>
    </div>
  );
}
