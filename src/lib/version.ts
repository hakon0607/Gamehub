/**
 * Version numbers, and the order releases are shown in.
 *
 * Pure, and tested: sorting versions as text is the classic bug that puts
 * 0.10.0 before 0.9.0 and quietly shows people an old download as the newest.
 */
import type { Release } from './types';

export type Parsed = [number, number, number];

export function parseVersion(value: string): Parsed | null {
  const match = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(String(value ?? '').trim());
  if (!match) return null;
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

/** Negative when a is older, positive when a is newer, 0 when equal. */
export function compareVersions(a: string, b: string): number {
  const left = parseVersion(a);
  const right = parseVersion(b);
  // Anything unparseable sorts last rather than throwing — a release with an
  // odd version should still be listed, just not claimed to be the newest.
  if (!left && !right) return a.localeCompare(b);
  if (!left) return -1;
  if (!right) return 1;
  for (let i = 0; i < 3; i += 1) {
    if (left[i] !== right[i]) return left[i] - right[i];
  }
  return 0;
}

/** Newest first. Does not mutate the input. */
export function sortReleases(releases: Release[]): Release[] {
  return [...releases].sort((a, b) => compareVersions(b.version, a.version));
}

/** The release the big download button points at. */
export function latestRelease(releases: Release[]): Release | null {
  const withFiles = releases.filter((r) => r.files.length > 0);
  return sortReleases(withFiles)[0] ?? null;
}

/** The file the download button points at: the installer, else the first. */
export function primaryFile(release: Release | null) {
  if (!release) return null;
  return release.files.find((f) => f.kind === 'installer') ?? release.files[0] ?? null;
}

/** Normalises "v1.2.3" and " 1.2.3 " to "1.2.3". */
export function normaliseVersion(value: string): string {
  const parsed = parseVersion(value);
  return parsed ? parsed.join('.') : String(value ?? '').trim();
}
