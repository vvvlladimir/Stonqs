// Records the IPC fixture for the website's screenshots without anybody clicking:
//
//   cd app && pnpm record:tour        then   pnpm screenshots in the stonqs-site repository
//
// Starts the dev app with the recorder and the tour on (src/lib/ipcTour.tsx), which opens a fresh
// demo profile and visits every screen, and stops the app once e2e/fixtures/ipc.json is marked
// complete. The dev identifier (app.stonqs.dev) keeps the "Screenshots" profile out of real data.

import { spawn } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";

const FIXTURE = "e2e/fixtures/ipc.json";
const TIMEOUT_MS = 15 * 60 * 1000;
const started = Date.now();

const app = spawn("pnpm", ["tauri", "dev", "--config", "src-tauri/tauri.dev.conf.json"], {
  env: { ...process.env, VITE_RECORD_IPC: "1", VITE_RECORD_TOUR: "1" },
  stdio: "inherit",
  // Its own process group, so stopping it also stops Vite and the Rust binary it started.
  detached: true,
});

function stop(code) {
  try {
    process.kill(-app.pid, "SIGTERM");
  } catch {}
  process.exit(code);
}

process.on("SIGINT", () => stop(130));
app.on("exit", (code) => {
  console.error(`tauri dev exited (${code}) before the tour finished`);
  process.exit(1);
});

function complete() {
  if (!existsSync(FIXTURE) || statSync(FIXTURE).mtimeMs < started) return false;
  try {
    return JSON.parse(readFileSync(FIXTURE, "utf8")).complete === true;
  } catch {
    return false; // caught mid-write
  }
}

const timer = setInterval(() => {
  if (complete()) {
    clearInterval(timer);
    const { calls } = JSON.parse(readFileSync(FIXTURE, "utf8"));
    console.log(`\nrecorded ${Object.keys(calls).length} answers into ${FIXTURE}`);
    app.removeAllListeners("exit");
    stop(0);
  } else if (Date.now() - started > TIMEOUT_MS) {
    console.error("the tour did not finish in time");
    stop(1);
  }
}, 1000);
