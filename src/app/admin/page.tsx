import { cookies } from 'next/headers';
import { readSession, SESSION_COOKIE, usingDefaultPassword } from '@/lib/auth';
import { blobConfigured, readManifest } from '@/lib/releases';
import { AdminClient } from './AdminClient';
import { LoginForm } from './LoginForm';

export const dynamic = 'force-dynamic';

export const metadata = { title: 'Admin — GameHub', robots: { index: false, follow: false } };

/**
 * The page is decided on the server. Rendering the dashboard and hiding it with
 * CSS would put the whole thing in the page source for anyone to read.
 */
export default async function AdminPage() {
  const store = await cookies();
  const session = readSession(store.get(SESSION_COOKIE)?.value);

  if (!session) {
    return <LoginForm />;
  }

  const manifest = await readManifest();

  return (
    <AdminClient
      initialReleases={manifest.releases}
      username={session.u}
      defaultPassword={usingDefaultPassword()}
      blobReady={blobConfigured()}
    />
  );
}
