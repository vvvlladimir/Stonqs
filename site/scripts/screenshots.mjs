// Re-shoots the product images on the website from the real frontend, in both colour schemes.
//
//   1. once, by hand:  cd app && pnpm record:ipc     (open the demo profile, visit the screens below)
//   2. any time:       cd site && pnpm screenshots
//
// Step 1 stores every host answer in app/e2e/fixtures/ipc.json. Step 2 serves the frontend with
// Vite, answers its IPC from that file inside Chrome (no Tauri, no Rust), and writes
// public/assets/<name>-{light,dark}.webp plus components/shots.json with their sizes.
// `pnpm screenshots --manifest` only rewrites shots.json from the images already there.

import { spawn } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import sharp from "sharp";

const FIXTURE = process.env.IPC_FIXTURE ?? "../app/e2e/fixtures/ipc.json";
// Overridable so a dry run does not touch the site's images.
const ASSETS = process.env.SHOTS_OUT ?? "public/assets";
const MANIFEST = "components/shots.json";
const PORT = 1421;
const VIEWPORT = { width: 1440, height: 860 };
const SCALE = 2;

/**
 * What to shoot. `screen` is the label typed into the command palette (English, the language the
 * replay forces); `region` picks a part of the screen, absent means the whole window, framed.
 */
const SHOTS = [
  { name: "overview", screen: "Overview" },
  { name: "kpis", screen: "Overview", region: tiles(0, 4) },
  { name: "chart", screen: "Overview", region: tileWith("Value and flows") },
  { name: "nav", screen: "Overview", region: (page) => box(page, ".nav") },
];

// ---- regions -------------------------------------------------------------------------------------

async function box(page, selector) {
  return page.evaluate((sel) => {
    const r = document.querySelector(sel)?.getBoundingClientRect();
    return r && { x: r.x, y: r.y, width: r.width, height: Math.min(r.height, innerHeight - r.y) };
  }, selector);
}

/** The union of dashboard tiles `from`..`to` in board order: the headline figures row. */
function tiles(from, to) {
  return (page) =>
    page.evaluate(
      ([a, b]) => {
        const rs = [...document.querySelectorAll(".w__body")]
          .slice(a, b)
          .map((el) => el.parentElement.getBoundingClientRect());
        if (!rs.length) return null;
        const x = Math.min(...rs.map((r) => r.left));
        const y = Math.min(...rs.map((r) => r.top));
        return { x, y, width: Math.max(...rs.map((r) => r.right)) - x, height: Math.max(...rs.map((r) => r.bottom)) - y };
      },
      [from, to],
    );
}

/** The dashboard tile whose text contains `text`. */
function tileWith(text) {
  return (page) =>
    page.evaluate((t) => {
      const tile = [...document.querySelectorAll(".w__body")]
        .map((el) => el.parentElement)
        .find((el) => el.textContent.includes(t));
      const r = tile?.getBoundingClientRect();
      return r && { x: r.x, y: r.y, width: r.width, height: r.height };
    }, text);
}

// ---- IPC replay (runs inside the page, before the app's own scripts) -----------------------------

function installReplay(fixture) {
  const calls = Object.values(fixture.calls);
  const exact = new Map(calls.map((c) => [`${c.command} ${JSON.stringify(c.args)}`, c]));
  const byCommand = new Map(calls.map((c) => [c.command, c]));
  const callbacks = new Map();
  let nextId = 1;
  window.__replay = { pending: 0, missing: new Set(), approximate: new Set() };

  function answer(c) {
    if (c.command === "settings_get" && c.ok) {
      // The shot decides language and scheme, not whatever the recording profile had.
      return { ...c.ok, language: "en", ui: { ...c.ok.ui, theme: "system" } };
    }
    if ("err" in c) throw c.err;
    return c.ok;
  }

  async function invoke(cmd, args) {
    window.__replay.pending++;
    try {
      await new Promise((r) => setTimeout(r, 0));
      const hit = exact.get(`${cmd} ${JSON.stringify(args ?? null)}`);
      if (hit) return answer(hit);
      if (cmd.startsWith("plugin:")) {
        if (cmd === "plugin:event|listen") return nextId++;
        if (cmd === "plugin:menu|new") return [nextId++, `menu-${nextId}`];
        if (cmd === "plugin:app|version") return "0.0.0";
        return null;
      }
      const near = byCommand.get(cmd);
      if (near) {
        window.__replay.approximate.add(cmd);
        return answer(near);
      }
      window.__replay.missing.add(cmd);
      throw { code: "internal", message: `not recorded: ${cmd}` };
    } finally {
      window.__replay.pending--;
    }
  }

  window.__TAURI_INTERNALS__ = {
    invoke,
    transformCallback(cb, once) {
      const id = nextId++;
      callbacks.set(id, (data) => {
        if (once) callbacks.delete(id);
        return cb?.(data);
      });
      return id;
    },
    unregisterCallback: (id) => callbacks.delete(id),
    convertFileSrc: (path) => path,
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
}

// ---- the window frame the webview has on macOS --------------------------------------------------

function frame(png, width, height, dark) {
  const bar = 28 * SCALE;
  const pad = 40 * SCALE;
  const radius = 12 * SCALE;
  const w = width + pad * 2;
  const h = height + bar + pad * 2;
  const chrome = dark ? { bar: "#1d1e22", line: "#2c2d33", text: "#e6e6ea" } : { bar: "#f3f3f5", line: "#dcdce1", text: "#1d1d22" };
  const lights = ["#ff5f57", "#febc2e", "#28c840"]
    .map((c, i) => `<circle cx="${pad + (20 + i * 20) * SCALE}" cy="${pad + bar / 2}" r="${6 * SCALE}" fill="${c}"/>`)
    .join("");
  const img = png.toString("base64");
  return Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${w}" height="${h}">
  <defs>
    <filter id="s" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="${14 * SCALE}"/></filter>
    <clipPath id="c"><rect x="${pad}" y="${pad}" width="${width}" height="${height + bar}" rx="${radius}"/></clipPath>
  </defs>
  <rect x="${pad}" y="${pad + 10 * SCALE}" width="${width}" height="${height + bar}" rx="${radius}" fill="#000" opacity="${dark ? 0.5 : 0.16}" filter="url(#s)"/>
  <g clip-path="url(#c)">
    <rect x="${pad}" y="${pad}" width="${width}" height="${bar}" fill="${chrome.bar}"/>
    <rect x="${pad}" y="${pad + bar - 1}" width="${width}" height="1" fill="${chrome.line}"/>
    <image x="${pad}" y="${pad + bar}" width="${width}" height="${height}" xlink:href="data:image/png;base64,${img}"/>
  </g>
  <rect x="${pad + 0.5}" y="${pad + 0.5}" width="${width - 1}" height="${height + bar - 1}" rx="${radius}" fill="none" stroke="${chrome.line}"/>
  ${lights}
  <text x="${pad + 84 * SCALE}" y="${pad + bar / 2 + 4.5 * SCALE}" font-family="-apple-system, Helvetica, sans-serif" font-size="${13 * SCALE}" font-weight="600" fill="${chrome.text}">Stonqs</text>
</svg>`);
}

// ---- run ----------------------------------------------------------------------------------------

async function writeManifest() {
  const sizes = {};
  for (const file of readdirSync(ASSETS).filter((f) => f.endsWith("-light.webp")).sort()) {
    const { width, height } = await sharp(`${ASSETS}/${file}`).metadata();
    sizes[file.replace("-light.webp", "")] = { width, height };
  }
  writeFileSync(MANIFEST, `${JSON.stringify(sizes, null, 2)}\n`);
  console.log(`wrote ${MANIFEST}`);
}

async function waitForServer(url) {
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(url)).ok) return;
    } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`Vite did not answer on ${url}`);
}

/** Quiet means no IPC in flight for a while: queries resolve in waves, a chart after its data. */
async function settle(page) {
  let quiet = 0;
  while (quiet < 6) {
    await page.waitForTimeout(100);
    quiet = (await page.evaluate(() => window.__replay.pending)) === 0 ? quiet + 1 : 0;
  }
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(400);
}

async function open(page, screen) {
  await page.keyboard.press("ControlOrMeta+KeyK");
  await page.keyboard.type(screen);
  await page.keyboard.press("Enter");
  await settle(page);
}

async function shoot() {
  if (!existsSync(FIXTURE)) {
    console.error(`No ${FIXTURE}. Record it first: cd app && pnpm record:ipc, then visit the screens.`);
    process.exit(1);
  }
  const fixture = JSON.parse(readFileSync(FIXTURE, "utf8"));
  const { chromium } = await import("playwright");

  const vite = spawn("pnpm", ["exec", "vite", "--port", String(PORT), "--strictPort"], {
    cwd: "../app",
    stdio: "ignore",
  });
  const url = `http://localhost:${PORT}/`;
  try {
    await waitForServer(url);
    const browser = await chromium.launch({ channel: "chrome" }).catch(() => chromium.launch());
    for (const scheme of ["light", "dark"]) {
      const context = await browser.newContext({
        viewport: VIEWPORT,
        deviceScaleFactor: SCALE,
        colorScheme: scheme,
        reducedMotion: "reduce",
      });
      const page = await context.newPage();
      // The recorded answers were computed for the recording's "today"; the page must agree.
      await page.clock.setFixedTime(new Date(fixture.recorded_at));
      await page.addInitScript(installReplay, fixture);
      await page.goto(url);
      await settle(page);

      let current = "Overview";
      for (const shot of SHOTS) {
        if (shot.screen !== current) {
          await open(page, shot.screen);
          current = shot.screen;
        }
        const out = `${ASSETS}/${shot.name}-${scheme}.webp`;
        if (shot.region) {
          const clip = await shot.region(page);
          if (!clip) {
            console.warn(`skip ${shot.name}: region not found on ${shot.screen}`);
            continue;
          }
          await sharp(await page.screenshot({ clip })).webp({ quality: 86 }).toFile(out);
        } else {
          const png = await page.screenshot();
          const svg = frame(png, VIEWPORT.width * SCALE, VIEWPORT.height * SCALE, scheme === "dark");
          await sharp(svg).resize({ width: 1800 }).webp({ quality: 86 }).toFile(out);
        }
        console.log(out);
      }

      const { missing, approximate } = await page.evaluate(() => ({
        missing: [...window.__replay.missing],
        approximate: [...window.__replay.approximate],
      }));
      if (missing.length) console.warn(`${scheme}: never recorded, answered with an error: ${missing.join(", ")}`);
      if (approximate.length) console.warn(`${scheme}: answered from other arguments: ${approximate.join(", ")}`);
      await context.close();
    }
    await browser.close();
  } finally {
    vite.kill();
  }
}

if (!process.argv.includes("--manifest")) await shoot();
if (!process.env.SHOTS_OUT) await writeManifest();
