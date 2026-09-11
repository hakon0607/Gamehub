/**
 * Automatic updates, through Tauri's official updater plugin.
 *
 * Everything here calls `@tauri-apps/plugin-updater` directly — `check()` for
 * the manifest and `update.downloadAndInstall()` for the download, which
 * verifies the bundle's signature against the public key compiled into the app
 * before anything is executed. There is no home-made update mechanism anywhere
 * in GameHub: if this file is deleted, updates simply stop.
 */
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';
import { t } from './i18n';

export interface UpdateStatus {
  available: boolean;
  version: string | null;
  currentVersion: string;
  /** Release notes from latest.json, when the release has any. */
  notes: string | null;
  date: string | null;
}

/** Turns the plugin's errors into something a person can act on. */
export function describeUpdateError(error: unknown): string {
  const text = String(error);
  const lower = text.toLowerCase();

  if (lower.includes('network') || lower.includes('dns') || lower.includes('connect') || lower.includes('timed out')) {
    return t('update.err_network');
  }
  if (lower.includes('signature') || lower.includes('verify')) {
    return t('update.err_signature');
  }
  if (lower.includes('404') || lower.includes('not found')) {
    return t('update.err_404');
  }
  if (lower.includes('endpoint') || lower.includes('config') || lower.includes('pubkey')) {
    return t('update.err_config');
  }
  return t('update.err_other', { error: text });
}

/**
 * Asks the endpoint whether something newer exists. Never installs anything.
 *
 * The `Update` handle is returned alongside the summary because
 * `downloadAndInstall` has to be called on the same object `check` produced.
 */
export async function checkForUpdate(): Promise<{ status: UpdateStatus; update: Update | null }> {
  const currentVersion = await getVersion();
  const update = await check();

  if (!update) {
    return {
      status: { available: false, version: null, currentVersion, notes: null, date: null },
      update: null,
    };
  }

  return {
    status: {
      available: true,
      version: update.version,
      currentVersion: update.currentVersion ?? currentVersion,
      notes: update.body?.trim() ? update.body : null,
      date: update.date ?? null,
    },
    update,
  };
}

/**
 * Downloads and installs, reporting progress as a percentage.
 *
 * On success GameHub relaunches, so nothing after `relaunch()` runs. Everything
 * in %APPDATA%\GameHub — library, covers, playtime, quests, settings — is left
 * alone by the installer.
 */
export async function downloadAndInstall(
  update: Update,
  onProgress: (percent: number) => void,
): Promise<void> {
  let downloaded = 0;
  let contentLength = 0;

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case 'Started':
        contentLength = event.data.contentLength ?? 0;
        onProgress(0);
        break;
      case 'Progress':
        downloaded += event.data.chunkLength;
        // A manifest without a content length still gets a moving bar rather
        // than a stuck one.
        onProgress(contentLength > 0 ? Math.min(100, Math.round((downloaded / contentLength) * 100)) : -1);
        break;
      case 'Finished':
        onProgress(100);
        break;
    }
  });

  await relaunch();
}
