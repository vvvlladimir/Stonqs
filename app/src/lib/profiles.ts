import { api } from "./api";

/** Set once a profile has been chosen in this window, so a reload does not ask again. */
// eslint-disable-next-line lingui/no-unlocalized-strings -- a storage key, not text
const CHOSEN = "vs.profile.chosen";

/** Whether the launch picker should be offered: several profiles and none chosen yet. */
export function needsPick(count: number): boolean {
  return count > 1 && sessionStorage.getItem(CHOSEN) === null;
}

export function markChosen(): void {
  sessionStorage.setItem(CHOSEN, "1");
}

/** Shortest password the host accepts (`vault::MIN_PASSWORD`). */
export const MIN_PASSWORD = 8;

/** Unlocking and locking change what every screen may read, so the window starts over. */
export function restart(): void {
  markChosen();
  window.location.reload();
}

/** Reloads rather than invalidating: every cached answer belongs to the profile being left. */
export async function openProfile(id: string, open: string): Promise<void> {
  markChosen();
  if (id === open) return;
  await api.profileOpen(id);
  window.location.reload();
}
