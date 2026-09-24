/**
 * Dev-only IPC recorder for the website's screenshots (site/scripts/screenshots.mjs). Started as
 * `pnpm record:ipc`; every command answer is sent to the Vite dev server, which keeps the latest
 * one per command and arguments in e2e/fixtures/ipc.json. Absent from a release build: the whole
 * module sits behind `import.meta.env.VITE_RECORD_IPC`, which only that script sets.
 */

export function recordIpc(command: string, args: Record<string, unknown> | undefined, outcome: Outcome) {
  // No keepalive: it caps a body at 64 KB, and a value series is larger than that.
  // eslint-disable-next-line lingui/no-unlocalized-strings -- a dev-server route, not text
  void fetch("/__ipc-record", {
    method: "POST",
    body: JSON.stringify({ command, args: args ?? null, ...outcome }),
  }).catch(() => {});
}

export type Outcome = { ok: unknown } | { err: unknown };
