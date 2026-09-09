/**
 * Downloads the ffmpeg build that ships inside the installer.
 *
 * Replay records by having ffmpeg write short rolling segments; without this
 * binary the Replay page says so and everything else works normally. The file
 * is not committed because it is ~80 MB.
 */
import { createWriteStream, existsSync, mkdirSync, readdirSync, renameSync, rmSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { execFileSync } from 'node:child_process';

const here = dirname(fileURLToPath(import.meta.url));
const target = join(here, '..', 'apps', 'desktop', 'src-tauri', 'resources');
const exe = join(target, 'ffmpeg.exe');
// ffprobe measures the buffer so a saved clip is the length that was asked for.
// Without it Replay still works, but every clip is the whole buffer.
const probe = join(target, 'ffprobe.exe');

// A build with gdigrab and libx264, published for Windows.
const URL = 'https://github.com/GyanD/codexffmpeg/releases/download/7.1/ffmpeg-7.1-essentials_build.zip';

if (existsSync(exe) && existsSync(probe)) {
  const mb = (statSync(exe).size / 1024 ** 2).toFixed(0);
  console.log(`\n  ffmpeg.exe and ffprobe.exe are already here (${mb} MB)\n  ${target}\n`);
  process.exit(0);
}

mkdirSync(target, { recursive: true });
const archive = join(target, 'ffmpeg.zip');

console.log('  Downloading ffmpeg (about 80 MB) …');
const response = await fetch(URL, { redirect: 'follow' });
if (!response.ok || !response.body) {
  console.error(`  Download failed: ${response.status} ${response.statusText}`);
  console.error('  Download it yourself from https://ffmpeg.org/download.html and put ffmpeg.exe in:');
  console.error(`  ${target}`);
  process.exit(1);
}
await pipeline(Readable.fromWeb(response.body), createWriteStream(archive));

console.log('  Unpacking …');
try {
  // tar has shipped with Windows 10 and 11 for years, and exists everywhere else.
  execFileSync('tar', ['-xf', archive, '-C', target], { stdio: 'inherit' });
} catch {
  console.error(`  Could not unpack ${archive}. Unzip it by hand and put ffmpeg.exe in ${target}.`);
  process.exit(1);
}

// The archive nests everything under a versioned folder; lift the one binary out.
const nested = readdirSync(target).find((entry) => entry.startsWith('ffmpeg-') && !entry.endsWith('.zip'));
if (nested) {
  for (const [name, destination] of [['ffmpeg.exe', exe], ['ffprobe.exe', probe]]) {
    const source = join(target, nested, 'bin', name);
    if (existsSync(source)) renameSync(source, destination);
  }
  rmSync(join(target, nested), { recursive: true, force: true });
}
rmSync(archive, { force: true });

if (!existsSync(exe)) {
  console.error(`  ffmpeg.exe did not end up in ${target}. Put it there by hand.`);
  process.exit(1);
}
if (!existsSync(probe)) {
  console.warn('  ffprobe.exe was not in the archive. Replay will still record and save,');
  console.warn('  but every clip will be the whole buffer rather than the last N seconds.');
}
console.log(`\n  ffmpeg ready — ${(statSync(exe).size / 1024 ** 2).toFixed(0)} MB\n  ${target}\n`);
