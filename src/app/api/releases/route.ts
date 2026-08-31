/**
 * The release list. GET is public; everything that changes it needs the cookie.
 */
import { NextResponse } from 'next/server';
import { cookies } from 'next/headers';
import { readSession, SESSION_COOKIE } from '@/lib/auth';
import { deleteFiles, readManifest, removeRelease, upsertRelease, writeManifest } from '@/lib/releases';
import { normaliseVersion, parseVersion } from '@/lib/version';
import type { Release, ReleaseFile } from '@/lib/types';

async function requireAdmin(): Promise<boolean> {
  const store = await cookies();
  return readSession(store.get(SESSION_COOKIE)?.value) !== null;
}

export async function GET() {
  const manifest = await readManifest();
  return NextResponse.json(manifest, {
    headers: { 'cache-control': 'no-store' },
  });
}

export async function POST(request: Request) {
  if (!(await requireAdmin())) {
    return NextResponse.json({ error: 'Du er ikke logget inn.' }, { status: 401 });
  }

  let body: {
    version?: string;
    name?: string;
    description?: string;
    files?: ReleaseFile[];
  };
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: 'Ugyldig forespørsel.' }, { status: 400 });
  }

  const version = normaliseVersion(String(body.version ?? ''));
  if (!parseVersion(version)) {
    return NextResponse.json(
      { error: 'Versjonen må se ut som 1.2.3 — tre tall med punktum mellom.' },
      { status: 400 },
    );
  }

  const files = Array.isArray(body.files) ? body.files : [];
  // A release with no file cannot be downloaded, and an empty row on the
  // download page is worse than a clear refusal here.
  if (files.length === 0) {
    return NextResponse.json({ error: 'Legg ved minst én fil.' }, { status: 400 });
  }
  for (const file of files) {
    if (!file?.url || !file?.name) {
      return NextResponse.json({ error: 'En av filene mangler navn eller adresse.' }, { status: 400 });
    }
  }

  const release: Release = {
    version,
    name: String(body.name ?? '').trim(),
    description: String(body.description ?? '').trim(),
    publishedAt: new Date().toISOString(),
    files: files.map((f) => ({
      name: String(f.name),
      url: String(f.url),
      size: Number(f.size) || 0,
      kind: f.kind === 'portable' || f.kind === 'other' ? f.kind : 'installer',
    })),
  };

  const manifest = await readManifest();
  const releases = upsertRelease(manifest.releases, release);
  await writeManifest({ ...manifest, releases });

  return NextResponse.json({ ok: true, releases });
}

export async function DELETE(request: Request) {
  if (!(await requireAdmin())) {
    return NextResponse.json({ error: 'Du er ikke logget inn.' }, { status: 401 });
  }

  const version = new URL(request.url).searchParams.get('version') ?? '';
  const manifest = await readManifest();
  const target = manifest.releases.find((r) => r.version === normaliseVersion(version));
  if (!target) {
    return NextResponse.json({ error: 'Fant ingen versjon med det nummeret.' }, { status: 404 });
  }

  // The list is written first. If deleting the files then fails, the release is
  // gone from the site — which is what was asked for — rather than the row
  // surviving with no files behind it.
  const releases = removeRelease(manifest.releases, target.version);
  await writeManifest({ ...manifest, releases });
  await deleteFiles(target.files.map((f) => f.url));

  return NextResponse.json({ ok: true, releases });
}
