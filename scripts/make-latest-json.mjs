/**
 * Builds the `latest.json` the updater reads.
 *
 * Getting this file wrong by hand is the usual reason updates silently do
 * nothing: a stale version, or a signature pasted with a line break in it. This
 * reads both straight out of the build output instead.
 *
 * Run it after `pnpm installer`, then upload the printed file to the release.
 */
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { looksLikeAPublicKey } from './apply-updater-pubkey.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const bundle = join(root, 'apps/desktop/src-tauri/target/release/bundle/nsis');
const config = JSON.parse(readFileSync(join(root, 'apps/desktop/src-tauri/tauri.conf.json'), 'utf8'));

const { version } = config;
const endpoint = config.plugins?.updater?.endpoints?.[0] ?? '';
// "https://github.com/OWNER/REPO/releases/latest/download/latest.json" -> OWNER/REPO
const repo = endpoint.match(/github\.com\/([^/]+\/[^/]+)\/releases/)?.[1];

if (!existsSync(bundle)) {
  console.error(`No build found at ${bundle}\nRun "pnpm installer" first.`);
  process.exit(1);
}
// Shares the check with apply-updater-pubkey.mjs so a leftover placeholder is
// caught here too, not just an empty string.
if (!looksLikeAPublicKey(config.plugins?.updater?.pubkey)) {
  console.error('tauri.conf.json has no real updater pubkey, so this build cannot be updated from.');
  console.error('Run: node scripts/apply-updater-pubkey.mjs --release');
  process.exit(1);
}

const installer = readdirSync(bundle).find((f) => f.endsWith(`${version}_x64-setup.exe`));
const signatureFile = installer && `${installer}.sig`;

if (!installer || !existsSync(join(bundle, signatureFile))) {
  console.error(
    `Could not find ${version} installer and .sig in ${bundle}.\n` +
      'Check that TAURI_SIGNING_PRIVATE_KEY was set when you built — without it Tauri skips the signature.',
  );
  process.exit(1);
}

const latest = {
  version,
  notes: process.argv.slice(2).join(' ') || `GameHub ${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: readFileSync(join(bundle, signatureFile), 'utf8').trim(),
      url: repo
        ? `https://github.com/${repo}/releases/download/v${version}/${installer}`
        : `REPLACE_WITH_THE_DOWNLOAD_URL_FOR/${installer}`,
    },
  },
};

const out = join(bundle, 'latest.json');
writeFileSync(out, `${JSON.stringify(latest, null, 2)}\n`);

console.log(`\n  latest.json written for ${version}\n  ${out}\n`);
// The Release workflow uploads these; they are listed so a log reader can see
// exactly what went into the release.
console.log('  Release assets:');
console.log(`    ${installer}`);
console.log(`    ${signatureFile}`);
console.log('    latest.json\n');
