/**
 * The app in a plain browser (`pnpm dev:browser`, `pnpm e2e`): every IPC call is forwarded to the
 * Rust browser host (`src-tauri/examples/browser_host`), which answers with the real command
 * functions over a demo portfolio. Loaded only behind `VITE_BROWSER_HOST`, before anything asks.
 *
 * What the host cannot run (a write needing the desktop window) comes back as a `UiError`, so a
 * screen shows its error state rather than an empty one; `e2e/` fails on a command nobody routed.
 */
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { markChosen } from "../profiles";

let resource = 1;

/** Tauri's own plugins, which have no host command behind them here. */
function plugin(command: string): unknown {
  switch (command) {
    case "plugin:app|version":
      return "0.0.0-browser";
    case "plugin:updater|check":
      return null;
    case "plugin:notification|is_permission_granted":
      return false;
    case "plugin:menu|new":
      return [resource++, `browser-${resource}`];
    default:
      return null;
  }
}

export function connectBrowserHost() {
  mockWindows("main");
  // The host holds the adopted first profile beside the demo one; the demo is already open.
  markChosen();
  mockIPC(
    async (command, payload) => {
      // eslint-disable-next-line lingui/no-unlocalized-strings -- Tauri's plugin prefix, not text
      if (command.startsWith("plugin:")) return plugin(command);
      const response = await fetch(`/__host/${command}`, {
        method: "POST",
        body: JSON.stringify(payload ?? {}),
      });
      const body = (await response.json()) as { ok: unknown } | { err: unknown };
      if ("err" in body) throw body.err;
      // The "system" language of a page is the browser's, which is how e2e picks a locale.
      if (command === "app_status") return { ...(body.ok as object), system_locale: navigator.language };
      return body.ok;
    },
    { shouldMockEvents: true },
  );
}
