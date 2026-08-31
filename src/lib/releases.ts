/**
 * The release list, kept as one JSON file in Vercel Blob.
 *
 * There is no database on purpose. The whole dataset is a handful of releases
 * that change when a human uploads one, so a single JSON blob read at request
 * time is both simpler and cheaper than anything else — and it means the site
 * has exactly one dependency to set up.
 */
import { put, list, del } from '@vercel/blob';
import { EMPTY_MANIFEST, type Manifest, type Release } from './types';
import { compareVersions, normaliseVersion, sortReleases } from './version';

const MANIFEST_PATH = 'releases.json';

export function blobConfigured(): boolean {
  return Boolean(process.env.BLOB_READ_WRITE_TOKEN);
}

/**
 * Reads the manifest. Never throws: a site that cannot reach Blob should still
 * render, showing no downloads rather than an error page.
 */
export async function readManifest(): Promise<Manifest> {
  if (!blobConfigured()) return { ...EMPTY_MANIFEST };

  try {
    // list() rather than a hardcoded URL, because the store's public hostname
    // is not known until the store exists.
    const found = await list({ prefix: MANIFEST_PATH, limit: 1 });
    const entry = found.blobs.find((b) => b.pathname === MANIFEST_PATH);
    if (!entry) return { ...EMPTY_MANIFEST };

    // no-store: the admin must see their change immediately after saving.
    const response = await fetch(entry.url, { cache: 'no-store' });
    if (!response.ok) return { ...EMPTY_MANIFEST };

    const parsed = (await response.json()) as Manifest;
    if (!parsed || !Array.isArray(parsed.releases)) return { ...EMPTY_MANIFEST };
    return { ...parsed, releases: sortReleases(parsed.releases) };
  } catch (error) {
    console.error('could not read the release list', error);
    return { ...EMPTY_MANIFEST };
  }
}

export async function writeManifest(manifest: Manifest): Promise<void> {
  const body = JSON.stringify(
    { ...manifest, schema: 1, updatedAt: new Date().toISOString() },
    null,
    2,
  );
  await put(MANIFEST_PATH, body, {
    access: 'public',
    contentType: 'application/json',
    // A fixed pathname, so the manifest always has the same address. Writing
    // the same pathname replaces what is there.
    addRandomSuffix: false,
    cacheControlMaxAge: 0,
  });
}

/**
 * Adds a release, or merges files into one that already exists.
 *
 * Pure so it can be tested: this is where a mistake would either lose an
 * existing release or list the same file twice.
 */
export function upsertRelease(releases: Release[], incoming: Release): Release[] {
  const version = normaliseVersion(incoming.version);
  const next = { ...incoming, version };

  const index = releases.findIndex((r) => compareVersions(r.version, version) === 0);
  if (index === -1) return sortReleases([...releases, next]);

  const existing = releases[index];
  // Same filename uploaded again replaces the old entry rather than appearing
  // twice; everything else is kept.
  const files = [...existing.files.filter((f) => !next.files.some((n) => n.name === f.name)), ...next.files];

  const merged: Release = {
    ...existing,
    name: next.name || existing.name,
    description: next.description || existing.description,
    publishedAt: next.publishedAt || existing.publishedAt,
    files,
  };

  const copy = [...releases];
  copy[index] = merged;
  return sortReleases(copy);
}

export function removeRelease(releases: Release[], version: string): Release[] {
  return releases.filter((r) => compareVersions(r.version, version) !== 0);
}

/** Deletes the uploaded files belonging to a release, ignoring ones already gone. */
export async function deleteFiles(urls: string[]): Promise<void> {
  if (urls.length === 0) return;
  try {
    await del(urls);
  } catch (error) {
    // A missing file is not a reason to refuse to remove the release from the
    // list — that would leave a row nobody can get rid of.
    console.error('some files could not be deleted', error);
  }
}
