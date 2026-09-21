import { useSyncExternalStore } from "react";
import type { DataChangeKind } from "./types";

/**
 * When each kind of data last changed, for the one thing a query cache cannot answer: whether a
 * *generated* artefact — an AI brief — was written before or after the figures under it moved.
 *
 * A query refetch is not a change (focus, reconnect, a stale timer all cause one), so this
 * listens to the host's own `data:changed` instead, the same event `affects` keys off. It lives
 * in memory on purpose: a marker persisted across restarts would have to be reconciled with
 * whatever happened while the app was closed, and the startup refresh announces that anyway.
 */
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

/**
 * Whether anything in `kinds` changed after `at`. An absent `at` is nothing to compare against,
 * which is not the same as stale.
 */
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
