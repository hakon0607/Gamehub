/** One downloadable file belonging to a release. */
export interface ReleaseFile {
  /** What the user sees, e.g. "GameHub-Setup.exe". */
  name: string;
  /** The public Blob URL it downloads from. */
  url: string;
  size: number;
  /**
   * "installer" is the one the download button points at; "portable" and
   * "other" are listed underneath.
   */
  kind: 'installer' | 'portable' | 'other';
}

export interface Release {
  /** "0.3.0" — the sort key, and what the page is titled by. */
  version: string;
  /** Optional friendly name, e.g. "Screenshots og sikkerhetskopier". */
  name: string;
  /** What changed. Plain text; blank lines separate paragraphs. */
  description: string;
  /** ISO 8601. */
  publishedAt: string;
  files: ReleaseFile[];
}

export interface Manifest {
  /** Bumped if the shape ever changes, so old data can be migrated. */
  schema: 1;
  releases: Release[];
  updatedAt: string;
}

export const EMPTY_MANIFEST: Manifest = {
  schema: 1,
  releases: [],
  updatedAt: '',
};
