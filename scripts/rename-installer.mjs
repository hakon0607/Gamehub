/**
 * Tauri names its NSIS output after the product and version —
 * `GameHub_0.1.0_x64-setup.exe`. That is a good archive name and a poor
 * download name, so this copies it to a stable `GameHub-Setup.exe` beside it.
 * The original is left in place: the updater manifest refers to it by its
 * versioned name.
 */
import { copyFileSync, existsSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const bundleDir = join(here, '..', 'apps', 'desktop', 'src-tauri', 'target', 'release', 'bundle', 'nsis');

if (!existsSync(bundleDir)) {
  console.error(`No installer found. Expected ${bundleDir}\nRun the build on Windows first.`);
  process.exit(1);
}

const installer = readdirSync(bundleDir).find((f) => f.endsWith('-setup.exe') && f !== 'GameHub-Setup.exe');
if (!installer) {
  console.error(`No *-setup.exe in ${bundleDir}`);
  process.exit(1);
}

const target = join(bundleDir, 'GameHub-Setup.exe');
copyFileSync(join(bundleDir, installer), target);
console.log(`\n  Installer ready:\n  ${target}\n`);
