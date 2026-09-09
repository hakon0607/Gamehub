/**
 * Tests for the updater public key resolution. Plain node, no framework —
 * the same shape as release-guard.test.mjs.
 */
import assert from 'node:assert/strict';
import { looksLikeAPublicKey, resolvePubkey, PLACEHOLDER } from './apply-updater-pubkey.mjs';

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

const RAW = 'untrusted comment: minisign public key A1B2C3\nRWQabcdefghijklmnop\n';
const ENCODED = Buffer.from(RAW).toString('base64');

test('recognises a raw minisign public key', () => {
  assert.equal(looksLikeAPublicKey(RAW), true);
});

test('recognises the base64 form the Tauri CLI prints', () => {
  assert.equal(looksLikeAPublicKey(ENCODED), true);
});

test('the placeholder is not a key', () => {
  assert.equal(looksLikeAPublicKey(PLACEHOLDER), false);
});

test('empty and missing are not keys', () => {
  assert.equal(looksLikeAPublicKey(''), false);
  assert.equal(looksLikeAPublicKey(undefined), false);
  assert.equal(looksLikeAPublicKey(null), false);
});

test('random text is not a key', () => {
  assert.equal(looksLikeAPublicKey('hunter2'), false);
});

test('the secret wins over whatever is in the config', () => {
  const result = resolvePubkey({ secret: ENCODED, current: 'something else', mode: 'release' });
  assert.equal(result.source, 'secret');
  assert.equal(result.pubkey, ENCODED);
});

test('surrounding whitespace from a paste is trimmed', () => {
  const result = resolvePubkey({ secret: `\n  ${ENCODED}  \n`, current: PLACEHOLDER, mode: 'release' });
  assert.equal(result.pubkey, ENCODED);
});

test('a real key already in the config is respected', () => {
  const result = resolvePubkey({ secret: undefined, current: RAW, mode: 'release' });
  assert.equal(result.source, 'config');
  assert.equal(result.pubkey, RAW.trim());
});

test('a set but malformed secret is refused rather than ignored', () => {
  const result = resolvePubkey({ secret: 'not-a-key', current: RAW, mode: 'release' });
  assert.ok(result.error, 'expected an error');
  assert.match(result.error, /does not look like/);
});

test('a release with no key anywhere fails with instructions', () => {
  const result = resolvePubkey({ secret: undefined, current: PLACEHOLDER, mode: 'release' });
  assert.ok(result.error, 'expected an error');
  assert.match(result.error, /TAURI_UPDATER_PUBKEY/);
  assert.match(result.error, /gamehub\.key\.pub/);
  assert.match(result.error, /Do not create a new key/);
});

test('check mode substitutes a throwaway key so CI can compile', () => {
  const result = resolvePubkey({ secret: undefined, current: PLACEHOLDER, mode: 'check' });
  assert.equal(result.source, 'throwaway');
  assert.equal(looksLikeAPublicKey(result.pubkey), true);
});

test('check mode still prefers a real key when one exists', () => {
  const result = resolvePubkey({ secret: ENCODED, current: PLACEHOLDER, mode: 'check' });
  assert.equal(result.source, 'secret');
});

console.log(failures === 0 ? '\nall updater key tests passed' : `\n${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
