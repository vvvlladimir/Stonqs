import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { lingui } from "@lingui/vite-plugin";

const IPC_FIXTURE = "e2e/fixtures/ipc.json";

/**
 * Collects what `pnpm record:ipc` sends (src/lib/ipcRecord.ts) into one fixture the website's
 * screenshot script replays. The latest answer per command and arguments wins, so clicking
 * through a screen twice does not grow the file.
 */
function ipcRecorder(): Plugin {
  return {
    name: "stonqs-ipc-recorder",
    apply: "serve",
    configureServer(server) {
      if (!process.env.VITE_RECORD_IPC) return;
      let fixture: { recorded_at: string; calls: Record<string, unknown> } = {
        recorded_at: new Date().toISOString(),
        calls: {},
      };
      try {
        fixture = { ...JSON.parse(readFileSync(IPC_FIXTURE, "utf8")), recorded_at: fixture.recorded_at };
      } catch {
        mkdirSync("e2e/fixtures", { recursive: true });
      }
      let timer: ReturnType<typeof setTimeout> | undefined;
      server.middlewares.use("/__ipc-record", (req, res) => {
        let body = "";
        req.on("data", (chunk) => (body += chunk));
        req.on("end", () => {
          const entry = JSON.parse(body);
          fixture.calls[`${entry.command} ${JSON.stringify(entry.args)}`] = entry;
          clearTimeout(timer);
          timer = setTimeout(() => writeFileSync(IPC_FIXTURE, JSON.stringify(fixture, null, 1)), 500);
          res.statusCode = 204;
          res.end();
        });
      });
    },
  };
}

export default defineConfig({
  // The macro plugin rewrites `<Trans>`/`t` at build time, so no message catalog is
  // looked up at runtime; `lingui()` compiles the imported `.po` catalogs.
  plugins: [react({ babel: { plugins: ["@lingui/babel-plugin-lingui-macro"] } }), lingui(), ipcRecorder()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/e2e/**"] },
  },
});
