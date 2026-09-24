// Re-shoots the product images on the website from the real frontend, in both colour schemes.
//
//   1. when the app changed:  cd app && pnpm record:tour    (fresh demo profile, every screen, no clicking)
//   2. any time:              cd site && pnpm screenshots
//
// Step 1 stores every host answer in app/e2e/fixtures/ipc.json. Step 2 serves the frontend with
// Vite, answers its IPC from that file inside Chrome (no Tauri, no Rust), and writes
// public/assets/<name>-{light,dark}.webp (plus a -800 copy of the wide ones), public/assets/og.jpg
// and components/shots.json with their sizes.
//
//   --only overview,income   shoot these and leave every other image as it is
//   --lenient                keep going when a screen asked something the fixture never recorded
//   --preview                every screen, whole window, into /tmp/stonqs-preview (for choosing a region)
//   --manifest               only rewrite shots.json from the images already there

import { spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import sharp from "sharp";

const FIXTURE = process.env.IPC_FIXTURE ?? "../app/e2e/fixtures/ipc.json";
// Overridable so a dry run does not touch the site's images.
const ASSETS = process.env.SHOTS_OUT ?? "public/assets";
const MANIFEST = "components/shots.json";
const PORT = 1421;
const DESKTOP = { width: 1440, height: 860, scale: 2 };
// The page is shorter than the phone by its status bar and home indicator, which the frame draws:
// Chrome reports no safe-area insets, so the app cannot leave room for them itself.
const PHONE = { width: 430, height: 932 - 54 - 22, scale: 3, top: 54, bottom: 22 };
/** Wide images get a second, 800px copy so a phone does not download 2,300 pixels. */
const SMALL = 800;

const arg = (name) => {
  const i = process.argv.indexOf(name);
  return i < 0 ? null : (process.argv[i + 1] ?? "");
};
const ONLY = arg("--only")?.split(",").filter(Boolean) ?? null;
const LENIENT = process.argv.includes("--lenient");
const PREVIEW = process.argv.includes("--preview");

/**
 * What to shoot. `screen` is the navigation label (English, the language the replay forces);
 * `region` picks a part of the screen, absent means the whole window in a frame; `before` runs
 * on the page first (a hover, a scroll); `device: "phone"` shoots the narrow layout in a phone.
 */
const SHOTS = [
  { name: "overview", screen: "Overview" },
  { name: "kpis", screen: "Overview", region: tiles(0, 4) },
  { name: "chart", screen: "Overview", region: tileWith("Value and flows"), before: hoverPlot(0.7) },
  { name: "nav", screen: "Overview", region: (page) => box(page, ".nav", 0) },
  { name: "performance", screen: "Performance", region: panelWith("Return by month") },
  { name: "allocation", screen: "Allocation", region: panelWith("Breakdown") },
  { name: "rebalance", screen: "Rebalance", region: panelWith("Deviation from target") },
  { name: "income", screen: "Income", region: panelWith("When the money arrives") },
  { name: "goals", screen: "Plans", region: panelWith("Goals") },
  { name: "mobile", screen: "Overview", device: "phone" },
];

/** Every screen the tour records (app/src/lib/ipcTour.tsx), for --preview. */
const SCREENS = ["Overview", "Positions", "Transactions", "Accounts", "Instruments", "Watchlist", "Plans", "Alerts", "Performance", "Trades", "Risk", "Allocation", "Rebalance", "Income", "Import", "Reports"];

// Things on screen that are true of the recording, not of the product: a tooltip the focus
// raised, the "viewing the past" banner, the dock floating over whatever panel is cropped.
const HIDE = `.tip, .timelens, .dock, .toast { display: none !important; }
*, *::before, *::after { caret-color: transparent !important; }`;

// ---- regions -------------------------------------------------------------------------------------

/** A rectangle with `pad` CSS pixels of the page around it, kept inside the window. Panels are
 *  cut flush (pad 0): any margin would catch the edge of the next panel, so the site frames them. */
function grow(r, pad, page) {
  const { width, height } = page.viewportSize();
  const x = Math.max(0, r.x - pad);
  const y = Math.max(0, r.y - pad);
  return {
    x,
    y,
    width: Math.min(width, r.x + r.width + pad) - x,
    height: Math.min(height, r.y + r.height + pad) - y,
  };
}

async function box(page, selector, pad = 16) {
  const r = await page.evaluate((sel) => {
    const b = document.querySelector(sel)?.getBoundingClientRect();
    return b && { x: b.x, y: b.y, width: b.width, height: Math.min(b.height, innerHeight - b.y) };
  }, selector);
  return r && grow(r, pad, page);
}

/** The union of boxes matched by `selector`, `from`..`to` in document order. */
function union(selector, from, to, pad = 0) {
  return async (page) => {
    const r = await page.evaluate(
      ([sel, a, b]) => {
        const rs = [...document.querySelectorAll(sel)].slice(a, b).map((el) => el.getBoundingClientRect());
        if (!rs.length) return null;
        const x = Math.min(...rs.map((r) => r.left));
        const y = Math.min(...rs.map((r) => r.top));
        const bottom = Math.min(innerHeight, Math.max(...rs.map((r) => r.bottom)));
        return { x, y, width: Math.max(...rs.map((r) => r.right)) - x, height: bottom - y };
      },
      [selector, from, to],
    );
    return r && grow(r, pad, page);
  };
}

/** Dashboard tiles `from`..`to` in board order: the headline figures row. */
function tiles(from, to) {
  return union(".w", from, to);
}

/** A screen's panels `from`..`to`, the top-level surfaces of `main`. */
function panels(from, to) {
  return union("main .box:not(.box .box)", from, to);
}

/** The smallest panel of the screen whose text contains `text`. */
function panelWith(text, pad = 0) {
  return async (page) => {
    const r = await page.evaluate((t) => {
      const b = [...document.querySelectorAll("main .box")]
        .filter((el) => el.textContent.includes(t))
        .map((el) => el.getBoundingClientRect())
        .sort((a, b) => a.width * a.height - b.width * b.height)[0];
      return b && { x: b.x, y: b.y, width: b.width, height: b.height };
    }, text);
    return r && grow(r, pad, page);
  };
}

/** The dashboard tile whose text contains `text`. */
function tileWith(text, pad = 0) {
  return async (page) => {
    const r = await page.evaluate((t) => {
      const tile = [...document.querySelectorAll(".w")].find((el) => el.textContent.includes(t));
      const b = tile?.getBoundingClientRect();
      return b && { x: b.x, y: b.y, width: b.width, height: b.height };
    }, text);
    return r && grow(r, pad, page);
  };
}

// ---- actions -------------------------------------------------------------------------------------

/** Rests the pointer at `at` (0..1) across the first chart's plot, so it shows its crosshair. */
function hoverPlot(at) {
  return async (page) => {
    const plot = page.locator(".chart__plot").first();
    const b = await plot.boundingBox();
    if (!b) return;
    await page.mouse.move(b.x + b.width * at, b.y + b.height * 0.5);
    await page.waitForTimeout(300);
  };
}

// ---- IPC replay (runs inside the page, before the app's own scripts) -----------------------------

function installReplay(fixture) {
  const calls = Object.values(fixture.calls);
  const exact = new Map(calls.map((c) => [`${c.command} ${JSON.stringify(c.args)}`, c]));
  const byCommand = new Map(calls.map((c) => [c.command, c]));
  const callbacks = new Map();
  let nextId = 1;
  window.__replay = { pending: 0, missing: new Set(), approximate: new Set() };
  // The recording has the "Screenshots" profile beside the dev one; the launch picker would ask.
  sessionStorage.setItem("vs.profile.chosen", "1");

  function answer(c) {
    if (c.command === "settings_get" && c.ok) {
      // The shot decides language and scheme, not whatever the recording profile had.
      return { ...c.ok, language: "en", ui: { ...c.ok.ui, theme: "system" } };
    }
    if ("err" in c) throw c.err;
    return c.ok;
  }

  // Writes the app makes on its own (a remembered column width, a "seen" mark) answer nothing.
  const WRITES = new Set(["ui_state_save", "alerts_mark_seen", "scope_set"]);

  async function invoke(cmd, args) {
    window.__replay.pending++;
    try {
      await new Promise((r) => setTimeout(r, 0));
      // The recorder saw `undefined` where Tauri's `invoke` passes `{}`.
      const key = JSON.stringify(args && Object.keys(args).length ? args : null);
      const hit = exact.get(`${cmd} ${key}`);
      if (hit) return answer(hit);
      if (cmd.startsWith("plugin:")) {
        if (cmd === "plugin:event|listen") return nextId++;
        if (cmd === "plugin:menu|new") return [nextId++, `menu-${nextId}`];
        if (cmd === "plugin:app|version") return "0.0.0";
        return null;
      }
      if (WRITES.has(cmd)) return null;
      const near = byCommand.get(cmd);
      if (near) {
        window.__replay.approximate.add(`${cmd} ${key}`);
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

// ---- frames --------------------------------------------------------------------------------------

/** The window the webview has on macOS, with its shadow. */
function windowFrame(png, width, height, scale, dark) {
  const bar = 28 * scale;
  const pad = 40 * scale;
  const radius = 12 * scale;
  const w = width + pad * 2;
  const h = height + bar + pad * 2;
  const chrome = dark
    ? { bar: "#1d1e22", line: "#2c2d33", text: "#e6e6ea" }
    : { bar: "#f3f3f5", line: "#dcdce1", text: "#1d1d22" };
  const lights = ["#ff5f57", "#febc2e", "#28c840"]
    .map((c, i) => `<circle cx="${pad + (20 + i * 20) * scale}" cy="${pad + bar / 2}" r="${6 * scale}" fill="${c}"/>`)
    .join("");
  return Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${w}" height="${h}">
  <defs>
    <filter id="s" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="${14 * scale}"/></filter>
    <clipPath id="c"><rect x="${pad}" y="${pad}" width="${width}" height="${height + bar}" rx="${radius}"/></clipPath>
  </defs>
  <rect x="${pad}" y="${pad + 10 * scale}" width="${width}" height="${height + bar}" rx="${radius}" fill="#000" opacity="${dark ? 0.5 : 0.16}" filter="url(#s)"/>
  <g clip-path="url(#c)">
    <rect x="${pad}" y="${pad}" width="${width}" height="${bar}" fill="${chrome.bar}"/>
    <rect x="${pad}" y="${pad + bar - 1}" width="${width}" height="1" fill="${chrome.line}"/>
    <image x="${pad}" y="${pad + bar}" width="${width}" height="${height}" xlink:href="data:image/png;base64,${png.toString("base64")}"/>
  </g>
  <rect x="${pad + 0.5}" y="${pad + 0.5}" width="${width - 1}" height="${height + bar - 1}" rx="${radius}" fill="none" stroke="${chrome.line}"/>
  ${lights}
  <text x="${pad + 84 * scale}" y="${pad + bar / 2 + 4.5 * scale}" font-family="-apple-system, Helvetica, sans-serif" font-size="${13 * scale}" font-weight="600" fill="${chrome.text}">Stonqs</text>
</svg>`);
}

/** A plain phone: bezel, rounded screen, island, status bar. Drawn, not a device photo, so it ages well. */
async function phoneFrame(png, device, dark) {
  const k = device.scale;
  const width = device.width * k;
  const top = device.top * k;
  const bottom = device.bottom * k;
  const height = device.height * k + top + bottom;
  // The status bar and the strip under the tab bar wear the colours the page has at those edges.
  const { data } = await sharp(png).raw().toBuffer({ resolveWithObject: true });
  const { channels } = await sharp(png).metadata();
  const px = (row) => {
    const i = (row * width + 4) * channels;
    return `rgb(${data[i]},${data[i + 1]},${data[i + 2]})`;
  };
  const head = px(0);
  const foot = px(device.height * k - 1);
  const ink = dark ? "#f1f1f4" : "#101116";
  const bezel = 12 * k;
  const pad = 36 * k;
  const radius = 58 * k;
  const w = width + (bezel + pad) * 2;
  const h = height + (bezel + pad) * 2;
  const x = pad + bezel;
  const y = pad + bezel;
  const body = dark ? "#2a2b30" : "#1b1c20";
  const bars = [0, 1, 2, 3]
    .map((i) => `<rect x="${x + width - 96 * k + i * 5 * k}" y="${y + (27 - i * 2.5) * k}" width="${3.4 * k}" height="${(4 + i * 2.5) * k}" rx="${0.8 * k}" fill="${ink}"/>`)
    .join("");
  return Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${w}" height="${h}">
  <defs>
    <filter id="s" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="${16 * k}"/></filter>
    <clipPath id="c"><rect x="${x}" y="${y}" width="${width}" height="${height}" rx="${radius - bezel}"/></clipPath>
  </defs>
  <rect x="${pad}" y="${pad + 12 * k}" width="${width + bezel * 2}" height="${height + bezel * 2}" rx="${radius}" fill="#000" opacity="${dark ? 0.55 : 0.22}" filter="url(#s)"/>
  <rect x="${pad}" y="${pad}" width="${width + bezel * 2}" height="${height + bezel * 2}" rx="${radius}" fill="${body}"/>
  <g clip-path="url(#c)">
    <rect x="${x}" y="${y}" width="${width}" height="${top}" fill="${head}"/>
    <image x="${x}" y="${y + top}" width="${width}" height="${device.height * k}" xlink:href="data:image/png;base64,${png.toString("base64")}"/>
    <rect x="${x}" y="${y + top + device.height * k}" width="${width}" height="${bottom}" fill="${foot}"/>
  </g>
  <text x="${x + 52 * k}" y="${y + 34 * k}" text-anchor="middle" font-family="-apple-system, Helvetica, sans-serif" font-size="${17 * k}" font-weight="600" fill="${ink}">9:41</text>
  ${bars}
  <rect x="${x + width - 70 * k}" y="${y + 21 * k}" width="${25 * k}" height="${12 * k}" rx="${3.5 * k}" fill="none" stroke="${ink}" stroke-opacity="0.4" stroke-width="${k}"/>
  <rect x="${x + width - 68 * k}" y="${y + 23 * k}" width="${21 * k}" height="${8 * k}" rx="${2 * k}" fill="${ink}"/>
  <rect x="${x + width / 2 - 62 * k}" y="${y + 11 * k}" width="${124 * k}" height="${36 * k}" rx="${18 * k}" fill="#000"/>
  <rect x="${x + width / 2 - 70 * k}" y="${y + height - 9 * k}" width="${140 * k}" height="${5 * k}" rx="${2.5 * k}" fill="${ink}"/>
</svg>`);
}

// ---- the social card -----------------------------------------------------------------------------

/** public/assets/og.jpg: the headline beside the real Overview, laid out as HTML in the same Chrome. */
async function socialCard(browser, dir) {
  const shot = join(dir, "overview-light.webp");
  if (!existsSync(shot)) return;
  const img = (await sharp(shot).png().toBuffer()).toString("base64");
  const icon = readFileSync("public/assets/icon-256.png").toString("base64");
  const font = readFileSync("public/assets/fonts/geist-latin-wght-normal.woff2").toString("base64");
  const page = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 });
  await page.setContent(`<!doctype html><style>
    @font-face { font-family: Geist; src: url(data:font/woff2;base64,${font}) format("woff2"); font-weight: 100 900; }
    body { margin: 0; width: 1200px; height: 630px; overflow: hidden; font-family: Geist, sans-serif;
      background: radial-gradient(900px 500px at 85% 20%, rgb(42 120 214 / 0.16), transparent 70%), #f5f5f7; color: #101116; }
    .copy { position: absolute; left: 72px; top: 96px; width: 440px; }
    .brand { display: flex; align-items: center; gap: 12px; font-size: 26px; font-weight: 600; letter-spacing: -0.01em; }
    h1 { margin: 44px 0 0; font-size: 56px; line-height: 1.04; letter-spacing: -0.035em; font-weight: 650; }
    p { margin: 24px 0 0; font-size: 22px; line-height: 1.4; color: #55565f; }
    .shot { position: absolute; left: 560px; top: 70px; width: 900px; }
  </style>
  <div class="copy">
    <div class="brand"><img src="data:image/png;base64,${icon}" width="40" height="40">Stonqs</div>
    <h1>Know how your investments are really doing.</h1>
    <p>Private, free and open source. Runs on your computer.</p>
  </div>
  <img class="shot" src="data:image/png;base64,${img}">`);
  await page.evaluate(() => document.fonts.ready);
  await sharp(await page.screenshot()).jpeg({ quality: 88, mozjpeg: true }).toFile(join(dir, "og.jpg"));
  await page.close();
  console.log("og.jpg");
}

// ---- run ----------------------------------------------------------------------------------------

async function writeManifest() {
  const sizes = {};
  for (const file of readdirSync(ASSETS).filter((f) => f.endsWith("-light.webp")).sort()) {
    const { width, height } = await sharp(`${ASSETS}/${file}`).metadata();
    const name = file.replace("-light.webp", "");
    sizes[name] = { width, height, ...(existsSync(`${ASSETS}/${name}-light-${SMALL}.webp`) && { small: SMALL }) };
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
  // A lazy screen can be between chunk and query with nothing in flight; its placeholder says so.
  await page
    .waitForFunction(() => !document.body.innerText.includes("Calculating…"), null, { timeout: 10000 })
    .catch(() => console.warn("still calculating after 10 s"));
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(400);
}

/** Opens a screen through the command palette: the same path at every width, no nav to find. */
async function open(page, screen) {
  await page.mouse.move(0, 0);
  await page.keyboard.press("ControlOrMeta+KeyK");
  await page.keyboard.type(screen);
  await page.waitForTimeout(150);
  // The screen row, not the first match: "Import" also finds the command "Import a file".
  await page
    .locator(".palette__item")
    .filter({ has: page.locator(".palette__label", { hasText: new RegExp(`^${screen}$`) }) })
    .first()
    .click();
  await settle(page);
  const title = await page.title();
  if (!title.startsWith(`${screen} ·`)) throw new Error(`the palette opened "${title}", not ${screen}`);
}

async function save(png, out) {
  const img = sharp(png);
  await img.clone().webp({ quality: 86 }).toFile(out);
  const { width } = await img.metadata();
  if (width > SMALL * 1.5) {
    await img.clone().resize({ width: SMALL }).webp({ quality: 84 }).toFile(out.replace(".webp", `-${SMALL}.webp`));
  }
}

async function shoot(page, shot, device, dark, out) {
  if (shot.before) await shot.before(page);
  if (shot.region) {
    const clip = await shot.region(page);
    if (!clip) throw new Error(`${shot.name}: region not found on ${shot.screen}`);
    return save(await page.screenshot({ clip }), out);
  }
  const png = await page.screenshot();
  const svg =
    shot.device === "phone"
      ? await phoneFrame(png, device, dark)
      : windowFrame(png, device.width * device.scale, device.height * device.scale, device.scale, dark);
  return save(await sharp(svg).resize({ width: shot.device === "phone" ? 900 : 1800 }).png().toBuffer(), out);
}

async function run() {
  if (!existsSync(FIXTURE)) {
    console.error(`No ${FIXTURE}. Record it first: cd app && pnpm record:tour`);
    process.exit(1);
  }
  const fixture = JSON.parse(readFileSync(FIXTURE, "utf8"));
  const { chromium } = await import("playwright");
  const shots = PREVIEW
    ? SCREENS.map((screen) => ({ name: screen.toLowerCase(), screen }))
    : SHOTS.filter((s) => !ONLY || ONLY.includes(s.name));
  if (ONLY && shots.length !== ONLY.length) {
    console.error(`unknown shot in --only; known: ${SHOTS.map((s) => s.name).join(", ")}`);
    process.exit(1);
  }
  // Written to a scratch folder first: a run that fails half way must not leave the site with a
  // mix of old and new images.
  const dir = PREVIEW ? "/tmp/stonqs-preview" : mkdtempSync(join(tmpdir(), "stonqs-shots-"));
  mkdirSync(dir, { recursive: true });

  const vite = spawn("pnpm", ["exec", "vite", "--port", String(PORT), "--strictPort"], { cwd: "../app", stdio: "ignore" });
  const url = `http://localhost:${PORT}/`;
  const problems = [];
  try {
    await waitForServer(url);
    const browser = await chromium.launch({ channel: "chrome" }).catch(() => chromium.launch());
    for (const kind of ["desktop", "phone"]) {
      const group = shots.filter((s) => (s.device ?? "desktop") === kind);
      if (!group.length) continue;
      const device = kind === "phone" ? PHONE : DESKTOP;
      for (const scheme of ["light", "dark"]) {
        const context = await browser.newContext({
          viewport: { width: device.width, height: device.height },
          deviceScaleFactor: device.scale,
          colorScheme: scheme,
          reducedMotion: "reduce",
          ...(kind === "phone" && { hasTouch: true, isMobile: true }),
        });
        const page = await context.newPage();
        // The recorded answers were computed for the recording's "today"; the page must agree.
        await page.clock.setFixedTime(new Date(fixture.recorded_at));
        await page.addInitScript(installReplay, fixture);
        await page.goto(url);
        await page.addStyleTag({ content: HIDE });
        await settle(page);

        let current = "Overview";
        for (const shot of group) {
          if (shot.screen !== current) {
            await open(page, shot.screen);
            current = shot.screen;
          }
          const out = join(dir, PREVIEW ? `${shot.name}-${scheme}.png` : `${shot.name}-${scheme}.webp`);
          if (PREVIEW) await page.screenshot({ path: out, fullPage: true });
          else await shoot(page, shot, device, scheme === "dark", out);
          console.log(out);
        }

        const { missing, approximate } = await page.evaluate(() => ({
          missing: [...window.__replay.missing],
          approximate: [...window.__replay.approximate],
        }));
        if (missing.length) problems.push(`${kind}/${scheme}: never recorded: ${missing.join(", ")}`);
        if (approximate.length) problems.push(`${kind}/${scheme}: recorded with other arguments: ${approximate.join(", ")}`);
        await context.close();
      }
    }
    if (!PREVIEW && (!ONLY || ONLY.includes("overview"))) await socialCard(browser, dir);
    await browser.close();
  } finally {
    vite.kill();
  }

  if (problems.length) {
    console.warn(problems.join("\n"));
    if (!LENIENT && !PREVIEW) {
      console.error(`\nNothing copied to ${ASSETS}: re-record with \`cd app && pnpm record:tour\`, or pass --lenient.`);
      process.exit(1);
    }
  }
  if (PREVIEW) return;
  for (const file of readdirSync(dir)) copyFileSync(join(dir, file), join(ASSETS, file));
  console.log(`copied ${readdirSync(dir).length} files to ${ASSETS}`);
}

if (!process.argv.includes("--manifest")) await run();
if (!process.env.SHOTS_OUT && !PREVIEW) await writeManifest();
