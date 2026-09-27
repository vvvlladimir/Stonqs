/** Dev-only IPC recorder for the site's screenshots, behind `VITE_RECORD_IPC`. */

export function recordIpc(command: string, args: Record<string, unknown> | undefined, outcome: Outcome) {
  // The tour's own setup (a profile deleted, a demo seeded) is not something a screen asks.
  if (import.meta.env.VITE_RECORD_TOUR && sessionStorage.getItem("vs.tour.stage") !== "visit") return;
  // No keepalive: it caps a body at 64 KB, and a value series is larger than that.
  // eslint-disable-next-line lingui/no-unlocalized-strings -- a dev-server route, not text
  void fetch("/__ipc-record", {
    method: "POST",
    body: JSON.stringify({ command, args: args ?? null, ...outcome }),
  }).catch(() => {});
}

export type Outcome = { ok: unknown } | { err: unknown };
