/**
 * Puts the updater public key into tauri.conf.json at build time.
 *
 * Why this exists: the public key belongs to whoever owns the signing key, and
 * a config file shipped in a zip must never be able to overwrite it. So the key
 * lives in the repository secret TAURI_UPDATER_PUBKEY and is written into the
 * config only for the length of a build. Nothing is committed, and a config
 * file arriving from anywhere else cannot clobber the real key.
 *
 * A real key already sitting in the config is respected and left alone, so a
 * normal local Tauri setup keeps working unchanged.
 *
 *   node scripts/apply-updater-pubkey.mjs --release   fail if no key is available
 *   node scripts/apply-updater-pubkey.mjs --check     substitute a throwaway key
 *
 * --check exists for CI, which compiles the app on pull requests where secrets
 * are not available. It prints a loud warning: such a build must never ship.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

export const PLACEHOLDER = 'SET_VIA_TAURI_UPDATER_PUBKEY_SECRET';

// A minisign public key is two lines: an untrusted comment, then base64. Tauri
// accepts it either raw or base64-encoded whole, which is how the CLI emits it.
export function looksLikeAPublicKey(value) {
  const text = String(value ?? '').trim();
  if (!text || text === PLACEHOLDER) return false;
  if (text.includes('minisign public key')) return true;
  // The whole two-line block, base64-encoded — what `tauri signer generate`
  // prints and what people normally paste.
  try {
    const decoded = Buffer.from(text, 'base64').toString('utf8');
    return decoded.includes('minisign public key');
  } catch {
    return false;
  }
}

/**
 * Decides what the pubkey field should become. Pure, so it can be tested
 * without touching the filesystem.
 */
export function resolvePubkey({ secret, current, mode }) {
  if (looksLikeAPublicKey(secret)) {
    return { pubkey: String(secret).trim(), source: 'secret' };
  }
  if (secret && String(secret).trim()) {
    return {
      error:
        'The TAURI_UPDATER_PUBKEY secret is set but does not look like a Tauri updater public key.\n' +
        'It should be the contents of gamehub.key.pub — a block that mentions "minisign public key",\n' +
        'or the base64 the Tauri CLI printed when the key was created. Check for a stray space or a\n' +
        'half-copied value, and set the secret again.',
    };
  }
  if (looksLikeAPublicKey(current)) {
    return { pubkey: String(current).trim(), source: 'config' };
  }
  if (mode === 'check') {
    // Structurally valid, owned by nobody. Compiles; cannot verify a real
    // update, which is exactly what a throwaway key should do.
    const throwaway = Buffer.from(
      'untrusted comment: minisign public key E0000000000000\nRWQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n',
    ).toString('base64');
    return { pubkey: throwaway, source: 'throwaway' };
  }
  return {
    error:
      'No updater public key is available, so this build could not be updated from.\n\n' +
      'Add a repository secret named TAURI_UPDATER_PUBKEY:\n' +
      '  Settings -> Secrets and variables -> Actions -> New repository secret\n\n' +
      'Its value is the contents of your existing public key file. To read it without a terminal,\n' +
      'open Notepad, choose File -> Open, type %USERPROFILE%\\.tauri\\gamehub.key.pub and press Enter\n' +
      '(set the file-type dropdown to All Files if it does not appear). Select all, copy, paste.\n\n' +
      'This is the .pub file, not the private key. It is the public half and is safe to hold.\n' +
      'Do not create a new key: every installed GameHub only trusts updates signed by the old one.',
  };
}

export function configPath(root) {
  return join(root, 'apps', 'desktop', 'src-tauri', 'tauri.conf.json');
}

function main() {
  const mode = process.argv.includes('--check') ? 'check' : 'release';
  const root = join(dirname(fileURLToPath(import.meta.url)), '..');
  const file = configPath(root);

  let config;
  const raw = readFileSync(file, 'utf8');
  try {
    config = JSON.parse(raw);
  } catch (error) {
    console.error(
      `::error title=tauri.conf.json is not valid JSON::${file} could not be parsed: ${error.message}`,
    );
    process.exit(1);
  }

  const updater = config?.plugins?.updater;
  if (!updater) {
    console.error(
      '::error title=Updater config missing::tauri.conf.json has no plugins.updater section, so the app cannot update itself.',
    );
    process.exit(1);
  }

  const result = resolvePubkey({
    secret: process.env.TAURI_UPDATER_PUBKEY,
    current: updater.pubkey,
    mode,
  });

  if (result.error) {
    // The annotation is one line because that is all the Actions summary shows;
    // the full text goes to the log underneath it.
    console.error(`::error title=No updater public key::${result.error.split('\n')[0]}`);
    console.error(`\n${result.error}\n`);
    process.exit(1);
  }

  updater.pubkey = result.pubkey;
  writeFileSync(file, `${JSON.stringify(config, null, 2)}\n`);

  if (result.source === 'throwaway') {
    console.log(
      '::warning title=Throwaway updater key::No TAURI_UPDATER_PUBKEY secret was available, so a throwaway key was used to let the app compile. This build must not be published.',
    );
  } else {
    console.log(`  Updater public key applied from the ${result.source}.`);
  }
  console.log(`  Endpoint: ${updater.endpoints?.[0] ?? '(none)'}`);
}

// Only run when executed directly, so the test file can import the functions.
if (process.argv[1] && process.argv[1].endsWith('apply-updater-pubkey.mjs')) {
  main();
}
