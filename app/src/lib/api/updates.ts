/** The updater. Its handle stays in this module: a screen decides, it never holds an installer. */

import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

/** The plugin's handle stays in this module; a screen never holds an installer. */
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

/** `null` is the normal answer; being offline throws rather than reading as up to date. */
export async function checkForUpdate(): Promise<AvailableUpdate | null> {
  const update = await check();
  pending = update;
  if (!update) return null;
  return { version: update.version, notes: update.body ?? "", date: update.date ?? null };
}

/** Progress is a fraction, or `null` without a content length. False when nothing is pending. */
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
