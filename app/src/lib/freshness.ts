import { useSyncExternalStore } from "react";
import type { DataChangeKind } from "./types";

/** When each data kind last changed per `data:changed`, for judging a generated brief; in memory only. */
const changedAt = new Map<DataChangeKind, number>();
const listeners = new Set<() => void>();

export function noteChange(kind: DataChangeKind) {
  changedAt.set(kind, Date.now());
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The most recent change among `kinds`, or 0 when none has been announced this session. */
function latest(kinds: DataChangeKind[]): number {
  return kinds.reduce((newest, kind) => Math.max(newest, changedAt.get(kind) ?? 0), 0);
}

/** An absent `at` is not stale. */
export function useChangedSince(at: string | undefined, kinds: DataChangeKind[]): boolean {
  const newest = useSyncExternalStore(
    subscribe,
    () => latest(kinds),
    () => 0,
  );
  if (!at) return false;
  const written = Date.parse(at);
  return Number.isFinite(written) && newest > written;
}
