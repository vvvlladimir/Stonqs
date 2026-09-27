/** What the host pushes: refresh progress, data changes, and OS notifications. */

import { listen } from "@tauri-apps/api/event";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import type { Progress, DataChanged, DataChangeKind } from "../types";

/** Subscribes to refresh progress shared by the whole application. */
export function onMarketProgress(handler: (progress: Progress) => void): Promise<() => void> {
  return listen<Progress>("market:progress", (event) => handler(event.payload));
}

/** The host says what it touched, so the frontend invalidates that group only. */
export function onDataChanged(handler: (kind: DataChangeKind) => void): Promise<() => void> {
  return listen<DataChanged>("data:changed", (event) => handler(event.payload.scope));
}

/** Shows an OS notification, asking for permission the first time. False when it is refused. */
export async function notify(title: string, body: string): Promise<boolean> {
  let granted = await isPermissionGranted();
  if (!granted) granted = (await requestPermission()) === "granted";
  if (granted) sendNotification({ title, body });
  return granted;
}
