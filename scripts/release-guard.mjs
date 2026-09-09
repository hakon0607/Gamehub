/**
 * Decides whether a push should become a release.
 *
 * Run on every push. It reads the version out of tauri.conf.json, compares it
 * against the tags that already exist, and either green-lights the build or
 * stops with an explanation of exactly what to change and where.
 *
 * Kept as a plain function so it can be tested without a GitHub runner.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** "1.2.10" -> [1, 2, 10]. Anything unparseable is rejected outright. */
export function parseVersion(value) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(String(value ?? '').trim());
  if (!match) return null;
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

/** Negative when a is older, 0 when equal, positive when a is newer. */
export function compareVersions(a, b) {
  for (let i = 0; i < 3; i += 1) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return 0;
}

/** The highest `vX.Y.Z` tag, ignoring anything that is not one. */
export function highestReleasedVersion(tags) {
  return tags
    .map((tag) => parseVersion(String(tag).replace(/^v/, '')))
    .filter(Boolean)
    .sort(compareVersions)
    .pop();
}

const format = (parts) => parts.join('.');

/**
 * @returns {{ok: true, version: string, tag: string} | {ok: false, reason: string}}
 */
export function decide({ version, tags, force = false }) {
  const parsed = parseVersion(version);
  if (!parsed) {
    return {
      ok: false,
      reason:
        `The version in apps/desktop/src-tauri/tauri.conf.json is "${version}", which is not a version ` +
        'number. It has to look like 0.2.1 — three numbers separated by dots.',
    };
  }

  const tag = `v${format(parsed)}`;
  const existing = tags.map((t) => String(t).trim()).filter(Boolean);

  if (existing.includes(tag) && !force) {
    return {
      ok: false,
      reason:
        `Release ${tag} already exists, so this push was not released again.\n\n` +
        'To publish a new version, open apps/desktop/src-tauri/tauri.conf.json on GitHub, ' +
        `click the pencil, and change "version": "${format(parsed)}" to the next number — ` +
        `for example "${format([parsed[0], parsed[1], parsed[2] + 1])}" — then commit.\n\n` +
        'If you really do mean to rebuild and replace this exact version, run the workflow ' +
        'manually from the Actions tab and tick "Replace the existing release".',
    };
  }

  const highest = highestReleasedVersion(existing);
  if (highest && compareVersions(parsed, highest) < 0 && !force) {
    return {
      ok: false,
      reason:
        `The version in tauri.conf.json is ${format(parsed)}, which is older than ${format(highest)} — ` +
        'already released. GameHub only updates forwards, so an older number would leave everyone ' +
        `stuck. Set the version to something above ${format(highest)}.`,
    };
  }

  return { ok: true, version: format(parsed), tag };
}

/** Reads the version tauri.conf.json declares — the single source of truth. */
export function versionFromConfig(root) {
  const path = join(root, 'apps', 'desktop', 'src-tauri', 'tauri.conf.json');
  return JSON.parse(readFileSync(path, 'utf8')).version;
}

// When run directly: argv[2] is a newline-separated list of existing tags.
if (import.meta.url === `file://${process.argv[1]}`) {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..');
  const version = versionFromConfig(root);
  const tags = (process.argv[2] ?? '').split(/\s+/).filter(Boolean);
  const force = process.argv[3] === 'force';

  const result = decide({ version, tags, force });

  if (!result.ok) {
    console.error(`\n${result.reason}\n`);
    // GitHub renders this as a red annotation at the top of the run.
    console.log(`::error title=Version not incremented::${result.reason.split('\n')[0]}`);
    process.exit(1);
  }

  console.log(`Releasing ${result.tag}`);
  if (process.env.GITHUB_OUTPUT) {
    const { appendFileSync } = await import('node:fs');
    appendFileSync(process.env.GITHUB_OUTPUT, `version=${result.version}\ntag=${result.tag}\n`);
  }
}
