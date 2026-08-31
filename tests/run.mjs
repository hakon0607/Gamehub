/**
 * Tests for the logic worth testing: version ordering, the login cookie, and
 * merging a release into the list.
 *
 * The TypeScript is transpiled with the compiler already in the project, so
 * there is no test framework to install and this runs anywhere Node does.
 */
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const require = createRequire(join(root, 'package.json'));
const ts = require('typescript');

const scratch = mkdtempSync(join(tmpdir(), 'gamehub-web-'));

/** Transpiles one src file, rewriting @/ imports to the files beside it. */
async function load(relative) {
  const source = readFileSync(join(root, 'src', relative), 'utf8');
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  const rewritten = outputText.replace(/from ['"]@\/lib\/([\w.-]+)['"]/g, "from './$1.mjs'");
  const file = join(scratch, `${relative.split('/').pop().replace(/\.ts$/, '')}.mjs`);
  writeFileSync(file, rewritten);
  return import(`file://${file}`);
}

// types.ts is imported for its constant; the interfaces vanish at runtime.
await load('lib/types.ts');
const version = await load('lib/version.ts');
const auth = await load('lib/auth.ts');
const format = await load('lib/format.ts');

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

console.log('\nVersions');

test('parses a plain version', () => {
  assert.deepEqual(version.parseVersion('1.2.3'), [1, 2, 3]);
});

test('accepts a leading v', () => {
  assert.deepEqual(version.parseVersion('v0.3.0'), [0, 3, 0]);
});

test('rejects nonsense', () => {
  assert.equal(version.parseVersion('latest'), null);
  assert.equal(version.parseVersion('1.2'), null);
  assert.equal(version.parseVersion(''), null);
});

test('compares numerically, not as text', () => {
  // The bug this guards: "0.10.0" < "0.9.0" as strings, which would show an
  // old build as the newest download on the front page.
  assert.ok(version.compareVersions('0.10.0', '0.9.0') > 0);
  assert.ok(version.compareVersions('1.0.0', '0.99.99') > 0);
  assert.equal(version.compareVersions('1.2.3', '1.2.3'), 0);
});

test('sorts releases newest first', () => {
  const list = [
    { version: '0.9.0', files: [] },
    { version: '0.10.0', files: [] },
    { version: '0.2.0', files: [] },
  ];
  assert.deepEqual(
    version.sortReleases(list).map((r) => r.version),
    ['0.10.0', '0.9.0', '0.2.0'],
  );
});

test('the newest release with a file is the one offered', () => {
  const list = [
    { version: '0.2.0', files: [{ name: 'a.exe', url: 'u', size: 1, kind: 'installer' }] },
    // Newer, but nothing to download — must not win the download button.
    { version: '0.3.0', files: [] },
  ];
  assert.equal(version.latestRelease(list).version, '0.2.0');
});

test('an empty list has no latest release', () => {
  assert.equal(version.latestRelease([]), null);
});

test('the download button prefers the installer over other files', () => {
  const release = {
    version: '1.0.0',
    files: [
      { name: 'portable.exe', url: 'p', size: 2, kind: 'portable' },
      { name: 'setup.exe', url: 's', size: 1, kind: 'installer' },
    ],
  };
  assert.equal(version.primaryFile(release).name, 'setup.exe');
});

test('with no installer it falls back to the first file', () => {
  const release = {
    version: '1.0.0',
    files: [{ name: 'portable.exe', url: 'p', size: 2, kind: 'portable' }],
  };
  assert.equal(version.primaryFile(release).name, 'portable.exe');
});

test('an odd version still sorts, it just never wins', () => {
  const list = [
    { version: 'nightly', files: [{ name: 'a', url: 'u', size: 1, kind: 'installer' }] },
    { version: '0.1.0', files: [{ name: 'b', url: 'u', size: 1, kind: 'installer' }] },
  ];
  assert.equal(version.latestRelease(list).version, '0.1.0');
});

test('normalises v1.2.3 and stray spaces', () => {
  assert.equal(version.normaliseVersion(' v1.2.3 '), '1.2.3');
});

console.log('\nLogin');

test('the right credentials are accepted', () => {
  assert.equal(auth.checkCredentials('admin', 'admin123'), true);
});

test('a wrong password is refused', () => {
  assert.equal(auth.checkCredentials('admin', 'admin124'), false);
});

test('a wrong username is refused', () => {
  assert.equal(auth.checkCredentials('root', 'admin123'), false);
});

test('a length mismatch does not throw', () => {
  // timingSafeEqual throws on differing lengths; the wrapper must not.
  assert.equal(auth.checkCredentials('a', 'x'), false);
  assert.equal(auth.checkCredentials('', ''), false);
});

test('a session round-trips', () => {
  const token = auth.createSession('admin', 'key');
  assert.equal(auth.readSession(token, 'key').u, 'admin');
});

test('a session signed with another key is rejected', () => {
  // The whole point: nobody can mint a cookie without the secret.
  const token = auth.createSession('admin', 'key');
  assert.equal(auth.readSession(token, 'other-key'), null);
});

test('a tampered payload is rejected', () => {
  const token = auth.createSession('admin', 'key');
  const forged = Buffer.from(JSON.stringify({ u: 'admin', exp: 9e9 })).toString('base64url');
  assert.equal(auth.readSession(`${forged}.${token.split('.')[1]}`, 'key'), null);
});

test('an expired session is rejected', () => {
  const token = auth.createSession('admin', 'key', 0);
  assert.equal(auth.readSession(token, 'key', Date.now()), null);
});

test('rubbish is rejected rather than throwing', () => {
  assert.equal(auth.readSession(undefined, 'key'), null);
  assert.equal(auth.readSession('', 'key'), null);
  assert.equal(auth.readSession('not-a-token', 'key'), null);
  assert.equal(auth.readSession('a.b.c', 'key'), null);
});

test('the default password is reported so the page can warn', () => {
  assert.equal(auth.usingDefaultPassword(), true);
});

console.log('\nFormatting');

test('sizes read the way people expect', () => {
  assert.equal(format.formatSize(0), '—');
  assert.equal(format.formatSize(900), '900 B');
  assert.equal(format.formatSize(1024 * 500), '500 kB');
  assert.equal(format.formatSize(1024 * 1024 * 90), '90.0 MB');
});

test('a bad date does not print "Invalid Date"', () => {
  assert.equal(format.formatDate('nonsense'), '');
});

test('a real date is Norwegian', () => {
  assert.equal(format.formatDate('2026-08-31T10:00:00Z'), '31. august 2026');
});

console.log(failures === 0 ? '\nall tests passed\n' : `\n${failures} failed\n`);
process.exit(failures === 0 ? 0 : 1);
