/**
 * Checks the app's Content Security Policy against what the app actually loads.
 *
 * A CSP is a deny-list by omission: anything a directive does not name is
 * refused, silently, with only a console message nobody sees. Cover images
 * worked because img-src names `asset:`; replay clips did not, because <video>
 * falls under default-src, which does not. The symptom was a video element that
 * stayed blank, and the workaround was opening the folder.
 *
 * Parsing the policy is enough to catch that class of mistake, and needs no
 * browser.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const config = JSON.parse(
  readFileSync(join(here, '..', 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8'),
);

const policy = config.app.security.csp ?? '';
const directives = Object.fromEntries(
  policy
    .split(';')
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const [name, ...values] = part.split(/\s+/);
      return [name, values];
    }),
);

let failures = 0;
function test(name, fn) {
  try {
    fn();
    console.log(`  ok  ${name}`);
  } catch (error) {
    failures += 1;
    console.error(`  FAIL  ${name}\n        ${error.message}`);
  }
}

/** What the browser actually consults for a given kind of load. */
function effective(directive) {
  return directives[directive] ?? directives['default-src'] ?? [];
}

const ASSET_SOURCES = ['asset:', 'http://asset.localhost'];

test('there is a policy at all', () => {
  assert.ok(policy.length > 0, 'no csp configured');
});

test('cover art and screenshots can load', () => {
  const sources = effective('img-src');
  for (const needed of ASSET_SOURCES) {
    assert.ok(sources.includes(needed), `img-src is missing ${needed}: ${sources.join(' ')}`);
  }
});

test('replay clips can play', () => {
  // The bug: <video> uses media-src, and media-src was not declared, so it fell
  // back to default-src, which has no asset: source.
  const sources = effective('media-src');
  for (const needed of ASSET_SOURCES) {
    assert.ok(
      sources.includes(needed),
      `media-src is missing ${needed} — videos will silently fail to load. Got: ${sources.join(' ')}`,
    );
  }
});

test('media-src is declared in its own right, not inherited', () => {
  assert.ok(
    'media-src' in directives,
    'media-src must be explicit; inheriting default-src is what broke video',
  );
});

test('the app can still talk to its own backend', () => {
  const sources = effective('connect-src');
  assert.ok(sources.includes('ipc:'), `connect-src is missing ipc:: ${sources.join(' ')}`);
});

test('scripts stay locked down', () => {
  // Loosening this would matter: the app launches programs.
  const sources = effective('script-src');
  assert.deepEqual(sources, ["'self'"], `script-src must stay 'self' only, got: ${sources.join(' ')}`);
  assert.ok(!sources.includes("'unsafe-inline'"));
  assert.ok(!sources.includes("'unsafe-eval'"));
});

test('nothing may be loaded from the open internet', () => {
  const remote = Object.entries(directives).filter(([, values]) =>
    values.some((v) => /^https?:\/\//.test(v) && !v.includes('localhost')),
  );
  assert.deepEqual(remote, [], `unexpected remote sources: ${JSON.stringify(remote)}`);
});

console.log(failures === 0 ? '\nall csp tests passed' : `\n${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
