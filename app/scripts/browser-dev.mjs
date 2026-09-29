// The app in a plain browser over a demo portfolio, no window and no Tauri build:
//
//   cd app && pnpm dev:browser        then open http://localhost:1420
//
// Starts the Rust browser host (src-tauri/examples/browser_host — the real command functions over
// HTTP) and Vite with VITE_BROWSER_HOST, which forwards every IPC call to it
// (src/lib/api/browserHost.ts). Writes that need the desktop window answer with an error.

import { spawn } from "node:child_process";

const env = { ...process.env, VITE_BROWSER_HOST: "1" };
const children = [
  spawn("cargo", ["run", "-p", "sq-app", "--example", "browser_host"], { env, stdio: "inherit" }),
  spawn("pnpm", ["exec", "vite"], { env, stdio: "inherit" }),
];

function stop(code) {
  for (const child of children) child.kill("SIGTERM");
  process.exit(code);
}

process.on("SIGINT", () => stop(130));
process.on("SIGTERM", () => stop(143));
for (const child of children) child.on("exit", (code) => stop(code ?? 1));
