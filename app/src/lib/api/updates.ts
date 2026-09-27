/** The updater. Its handle stays in this module: a screen decides, it never holds an installer. */

import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

/**
 * What an available update says about itself. The plugin's own handle stays in this module: a
 * screen decides whether to install, it never holds an installer.
 */
export interface AvailableUpdate {
  version: string;
  /** The release body, as markdown. */
  notes: string;
  /** Publication date exactly as the release names it; absent for a release without one. */
  date: string | null;
}

/** The running build's own version, as the bundle declares it. */
export function appVersion(): Promise<string> {
  return getVersion();
}

let pending: Update | null = null;

/**
 * Asks the release feed whether something newer is signed and published. `null` is the normal
 * answer. Throws when the feed cannot be reached — being offline is not "up to date".
 */
export async function checkForUpdate(): Promise<AvailableUpdate | null> {
  const update = await check();
  pending = update;
  if (!update) return null;
  return { version: update.version, notes: update.body ?? "", date: update.date ?? null };
}

/**
 * Downloads and installs what the last check found, reporting how much has arrived: a fraction
 * while the size is known, `null` while it is not, because a server may send no content length.
 * False when there is nothing pending — the check was superseded, so it is asked again.
 */
export async function installUpdate(onProgress: (done: number | null) => void): Promise<boolean> {
  const update = pending;
  if (!update) return false;
  let total = 0;
  let got = 0;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") total = event.data.contentLength ?? 0;
    else if (event.event === "Progress") {
      got += event.data.chunkLength;
      onProgress(total > 0 ? got / total : null);
    } else onProgress(1);
  });
  return true;
}

/** Restarts into the version just installed. */
export function restart(): Promise<void> {
  return relaunch();
}
