/** The one door to the host: every command goes through `call`. */

import { invoke } from "@tauri-apps/api/core";
import type { DataScope, DateString, UiError } from "../types";
import type { Outcome } from "../ipcRecord";

// Folded to `undefined` unless recording, so a normal build does not even carry the chunk.
const record = import.meta.env.VITE_RECORD_IPC
  ? async (command: string, args: Record<string, unknown> | undefined, outcome: Outcome) => {
      const { recordIpc } = await import("../ipcRecord");
      recordIpc(command, args, outcome);
    }
  : undefined;

/** Normalizes serialized host errors while preserving their structured detail. */
export class ApiError extends Error {
  constructor(readonly detail: UiError) {
    super(detail.message);
    this.name = "ApiError";
  }
}

function isUiError(value: unknown): value is UiError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const result = await invoke<T>(command, args);
    void record?.(command, args, { ok: result });
    return result;
  } catch (raw) {
    void record?.(command, args, { err: raw });
    if (isUiError(raw)) throw new ApiError(raw);
    throw new ApiError({ code: "internal", message: String(raw) });
  }
}

/**
 * The data source a reporting call is answered in. Absent — the usual case — means the one the
 * picker holds; a dashboard widget names its own so a board can carry a tile per account.
 */
export type Source = DataScope | null | undefined;

/** Returns today's local calendar date without timezone shifting. */
export function today(): DateString {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}
