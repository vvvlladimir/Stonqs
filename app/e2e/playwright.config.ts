import { defineConfig, devices } from "@playwright/test";

// Ports of their own, so `pnpm e2e` runs beside `pnpm tauri dev` (1420) and `pnpm dev:browser` (1430).
const HOST_PORT = "1431";
const WEB_PORT = "1441";
const CI = !!process.env.CI;
// Not `dist`: that is the desktop build's frontend, which this one must never replace.
const DIST = "e2e/.dist";

/**
 * Every screen, opened in a real browser engine against the real host's commands over a demo
 * portfolio (`src-tauri/examples/browser_host`). WebKit is the engine that matters — WKWebView on
 * macOS and iOS, WebKitGTK on Linux — and Chromium stands in for WebView2 on Windows.
 */
export default defineConfig({
  testDir: ".",
  // Every screen is its own test in one file, so without this they would queue on one worker.
  fullyParallel: true,
  workers: "75%",
  forbidOnly: CI,
  outputDir: "../test-results",
  reporter: CI ? [["github"], ["html", { open: "never", outputFolder: "../playwright-report" }]] : [["list"]],
  timeout: 60_000,
  use: { baseURL: `http://localhost:${WEB_PORT}`, trace: "retain-on-failure" },
  webServer: [
    {
      command: "cargo run -p sq-app --example browser_host",
      cwd: "..",
      env: { BROWSER_HOST_PORT: HOST_PORT },
      url: `http://127.0.0.1:${HOST_PORT}/__host/ping`,
      // A cold build of the host is minutes, not seconds.
      timeout: 15 * 60_000,
      reuseExistingServer: !CI,
      stdout: "pipe",
    },
    {
      // A build rather than the dev server, which serves every page its modules one by one and
      // becomes the bottleneck at a few pages at once. `development` keeps React's warnings.
      command: `pnpm exec vite build --mode development --outDir ${DIST} --emptyOutDir && pnpm exec vite preview --outDir ${DIST} --port ${WEB_PORT} --strictPort`,
      cwd: "..",
      env: { VITE_BROWSER_HOST: "1", BROWSER_HOST_PORT: HOST_PORT, NODE_ENV: "development" },
      url: `http://localhost:${WEB_PORT}`,
      reuseExistingServer: !CI,
    },
  ],
  projects: [
    { name: "webkit", use: { ...devices["Desktop Safari"], locale: "en-US" } },
    // Russian runs longest, so it is what finds a label that no longer fits.
    { name: "webkit-ru", use: { ...devices["Desktop Safari"], locale: "ru-RU" } },
    { name: "chromium", use: { ...devices["Desktop Chrome"], locale: "en-US" } },
  ],
});
