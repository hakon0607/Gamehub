/**
 * The legal documents have to agree with the code and with each other.
 *
 * A privacy policy that promises something the app does not do is worse than
 * no policy at all, so the few claims that can be checked mechanically are
 * checked here, and this runs as part of `pnpm test:release`.
 */
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import assert from 'node:assert/strict';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const legalDir = join(root, 'apps/desktop/src/legal');
const read = (p) => readFileSync(join(root, p), 'utf8');

const DOCS = [
  'terms-of-service.en.md',
  'terms-of-service.nb.md',
  'privacy-policy.en.md',
  'privacy-policy.nb.md',
  'third-party-notices.md',
  'license.md',
];

const rust = read('apps/desktop/src-tauri/src/legal.rs');
const version = /LEGAL_VERSION: &str = "([^"]+)"/.exec(rust)?.[1];
const date = /LEGAL_DATE: &str = "([^"]+)"/.exec(rust)?.[1];

test('every document ships with the app', () => {
  for (const doc of DOCS) {
    assert.ok(existsSync(join(legalDir, doc)), `${doc} is missing`);
  }
});

test('the documents carry the version the code says they do', () => {
  assert.ok(version, 'LEGAL_VERSION not found in legal.rs');
  const pretty = new Date(`${date}T00:00:00Z`).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
    timeZone: 'UTC',
  });
  for (const doc of ['terms-of-service.en.md', 'privacy-policy.en.md']) {
    const text = readFileSync(join(legalDir, doc), 'utf8');
    assert.match(text, new RegExp(`\\*\\*Version ${version} — ${pretty}\\*\\*`), `${doc} must say "Version ${version} — ${pretty}"`);
  }
});

test('the same contact address appears everywhere it is promised', () => {
  const email = 'hakon.solvik@hotmail.com';
  for (const doc of DOCS.filter((d) => d !== 'license.md')) {
    assert.ok(readFileSync(join(legalDir, doc), 'utf8').includes(email), `${doc} must name the contact address`);
  }
  const en = JSON.parse(read('apps/desktop/src/i18n/en.json'));
  assert.ok(en['legal.contact_body'].includes(email), 'the contact key must name the address');
});

test('the licence the app ships is the licence the repository grants', () => {
  assert.equal(read('LICENSE').trim(), readFileSync(join(legalDir, 'license.md'), 'utf8').trim());
  assert.match(read('LICENSE'), /MIT License/);
  assert.match(read('LICENSE'), /Håkon Solvik/);
});

test('the GPL obligation for the bundled ffmpeg is written down', () => {
  const notices = readFileSync(join(legalDir, 'third-party-notices.md'), 'utf8');
  const fetcher = read('scripts/fetch-ffmpeg.mjs');
  const url = /const URL = '([^']+)'/.exec(fetcher)?.[1];
  assert.ok(url, 'the ffmpeg download URL should be readable from the fetch script');
  assert.ok(notices.includes(url), 'third-party-notices.md must name the exact ffmpeg build that ships');
  assert.match(notices, /General Public License/, 'the GPL must be named');
  assert.match(notices, /source/i, 'there must be an offer of source code');
});

test('statistics are off until consent, in the code and not only on paper', () => {
  const store = read('apps/desktop/src-tauri/src/store.rs');
  assert.match(store, /stats_consent: false|LegalSettings::default\(\)/, 'the default must not consent');
  const defaults = /impl Default for LegalSettings \{[\s\S]*?stats_consent: (\w+)/.exec(rust)?.[1];
  assert.equal(defaults, 'false', 'LegalSettings must default to no consent');
  const telemetry = read('apps/desktop/src-tauri/src/telemetry.rs');
  assert.match(telemetry, /if !consented\(state\) \{\s*return;/, 'send_once must return early without consent');
});

test('the privacy policy lists every field the app actually sends', () => {
  const telemetry = read('apps/desktop/src-tauri/src/telemetry.rs');
  // Only the message itself: `payload` is the one function whose json! block
  // becomes the body that leaves the machine.
  const body = /pub fn payload\([\s\S]*?serde_json::json!\(\{([\s\S]*?)\n    \}\)/.exec(telemetry)?.[1] ?? '';
  assert.ok(body.length > 50, 'could not read the payload body out of telemetry.rs');
  const fields = [...body.matchAll(/"(\w+)":/g)].map((m) => m[1]);
  const policy = readFileSync(join(legalDir, 'privacy-policy.en.md'), 'utf8').toLowerCase();
  const described = {
    id: 'installation id',
    version: 'app version',
    language: 'language',
    state: 'window state',
    playing: 'the game running right now',
    games: 'number of installed games',
    launchers: 'launchers',
    os: 'windows version',
    events: 'feature counts',
  };
  for (const field of new Set(fields)) {
    assert.ok(described[field], `telemetry sends "${field}" but the policy check does not know about it`);
    assert.ok(policy.includes(described[field]), `the privacy policy must describe "${field}"`);
  }
});

test('the features the app counts are the ones the website accepts', () => {
  const telemetry = read('apps/desktop/src-tauri/src/telemetry.rs');
  const inApp = /pub const EVENTS: \[&str; \d+\] = \[([^\]]+)\]/.exec(telemetry)?.[1];
  const names = [...inApp.matchAll(/"(\w+)"/g)].map((m) => m[1]).sort();
  const site = readFileSync('/home/claude/gamehub-nettside/app/api/ping/route.ts', 'utf8');
  const accepted = [...(/const EVENTS = new Set\(\[([^\]]+)\]/.exec(site)?.[1] ?? '').matchAll(/'(\w+)'/g)]
    .map((m) => m[1])
    .sort();
  assert.deepEqual(names, accepted, 'app and website must agree on the feature names');
});
