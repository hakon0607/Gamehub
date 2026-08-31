import type { Metadata } from 'next';
import Link from 'next/link';
import './globals.css';

export const metadata: Metadata = {
  title: 'GameHub — alle PC-spillene dine på ett sted',
  description:
    'GameHub samler spillene dine fra Steam, Epic, Xbox, EA, Ubisoft Connect, Battle.net, GOG og Riot i ett bibliotek. Gratis for Windows.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="nb">
      <body>
        <header className="top">
          <div className="wrap">
            <Link href="/" className="brand">
              <span className="mark" aria-hidden="true">
                ◆
              </span>
              GameHub
            </Link>
            <nav className="links">
              <Link href="/last-ned">Last ned</Link>
              <Link href="/versjoner">Versjoner</Link>
            </nav>
          </div>
        </header>

        <main>{children}</main>

        <footer className="bottom">
          <div className="wrap">
            <span>GameHub — gratis, og alt ligger på din egen PC.</span>
            <Link href="/admin" style={{ color: 'inherit' }}>
              Admin
            </Link>
          </div>
        </footer>
      </body>
    </html>
  );
}
