#!/usr/bin/env node
// Generates THIRD-PARTY-NOTICES.md: every dependency that ends up in a build, with the licence
// text it ships under. MIT, BSD and ISC require their notice to travel with the binary, and the
// binary is what a user gets — so the file is generated from the lockfiles rather than written.
//
//   node scripts/notices.mjs            # rewrite THIRD-PARTY-NOTICES.md
//   node scripts/notices.mjs --check    # fail if the file is out of date (CI)
//
// Identical licence texts are printed once and shared: two hundred MIT crates differ only in
// their copyright line, and that line is part of the text, so nothing is dropped by grouping.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "THIRD-PARTY-NOTICES.md");
// The list without the texts, for the table the app shows. The texts stay out of the frontend
// bundle: a megabyte of licences is a download, not a screen.
const LIST = join(ROOT, "app", "src", "generated", "notices.json");
// How many of each there are, on its own so the summary row costs the panel nothing: the list
// itself is a hundred kilobytes and is fetched only when somebody asks to see it.
const COUNTS = join(ROOT, "app", "src", "generated", "counts.ts");

/** File names a package puts its licence in, most specific first. */
const LICENCE_FILES = /^(licen[cs]e|copying|notice|unlicen[cs]e)([-_.].*)?$/i;

function run(command, args, cwd) {
  return execFileSync(command, args, { cwd, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
}

/** Every licence-ish file in a package directory, read whole. Subdirectories are not searched:
 *  a `licenses/` folder belongs to what that package vendors, not to what it is. */
function licenceTexts(dir) {
  let names;
  try {
    names = readdirSync(dir);
  } catch {
    return [];
  }
  const out = [];
  for (const name of names.sort()) {
    if (!LICENCE_FILES.test(name)) continue;
    const path = join(dir, name);
    try {
      if (!statSync(path).isFile()) continue;
      const text = readFileSync(path, "utf8").replace(/\r\n/g, "\n").trim();
      if (text) out.push(text);
    } catch {
      // A package may name a licence file it does not ship; the SPDX id still records the terms.
    }
  }
  return out;
}

/** Rust: every crate reachable from the workspace other than through a dev-dependency. A build
 *  dependency stays in — a proc macro's output is part of the binary it expanded into. */
function crates() {
  const meta = JSON.parse(run("cargo", ["metadata", "--format-version", "1"], ROOT));
  const packages = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const members = new Set(meta.workspace_members);

  const seen = new Set();
  const queue = [...members];
  while (queue.length) {
    const id = queue.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (dep.dep_kinds.every((k) => k.kind === "dev")) continue;
      queue.push(dep.pkg);
    }
  }

  return [...seen]
    .filter((id) => !members.has(id))
    .map((id) => packages.get(id))
    .filter(Boolean)
    .map((p) => ({
      name: p.name,
      version: p.version,
      licence: p.license ?? (p.license_file ? `see ${p.license_file}` : "unstated"),
      url: p.repository ?? `https://crates.io/crates/${p.name}`,
      texts: licenceTexts(dirname(p.manifest_path)),
    }))
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
}

/** JavaScript: what pnpm resolves for the app's runtime dependencies. The bundle is a subset of
 *  this — over-listing a package costs a paragraph, under-listing one drops a required notice. */
function packages() {
  const app = join(ROOT, "app");
  const listed = JSON.parse(run("pnpm", ["licenses", "list", "--json", "--prod"], app));
  const out = [];
  for (const [licence, entries] of Object.entries(listed)) {
    for (const entry of entries) {
      const paths = entry.paths ?? [];
      out.push({
        name: entry.name,
        version: (entry.versions ?? []).join(", "),
        licence: entry.license ?? licence,
        url: entry.homepage ?? `https://www.npmjs.com/package/${entry.name}`,
        texts: paths.flatMap((p) => licenceTexts(p)),
      });
    }
  }
  return out.sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
}

function table(rows) {
  const lines = ["| Package | Version | Licence |", "| --- | --- | --- |"];
  for (const r of rows) lines.push(`| [${r.name}](${r.url}) | ${r.version} | ${r.licence} |`);
  return lines.join("\n");
}

/** One block per distinct licence text, naming everything that ships under exactly that text. */
function texts(groups) {
  const byHash = new Map();
  for (const item of groups) {
    for (const text of item.texts) {
      const key = createHash("sha256").update(text).digest("hex");
      const held = byHash.get(key) ?? { text, users: [] };
      held.users.push(`${item.name} ${item.version}`);
      byHash.set(key, held);
    }
  }
  return [...byHash.values()]
    .map((g) => ({ ...g, users: [...new Set(g.users)].sort() }))
    .sort((a, b) => a.users[0].localeCompare(b.users[0]))
    .map((g) => `### ${g.users.join(", ")}\n\n\`\`\`text\n${g.text}\n\`\`\``)
    .join("\n\n");
}

const rust = crates();
const js = packages();

const document = `# Third-party notices

Stonqs is licensed under AGPL-3.0-only. It builds on the work below, which keeps its own terms:
MIT, Apache-2.0, BSD and ISC all ask that their licence text and copyright notice travel with any
copy, including a compiled one. This file is that notice, and it ships with every release.

Generated by \`node scripts/notices.mjs\` from the lockfiles — do not edit it by hand. A licence
text shared by several packages is printed once and lists everything that uses it.

## Rust crates (${rust.length})

${table(rust)}

## JavaScript packages (${js.length})

${table(js)}

## Licence texts

${texts([...rust, ...js])}
`;

const strip = ({ name, version, licence, url }) => ({ name, version, licence, url });
const list = `${JSON.stringify({ rust: rust.map(strip), js: js.map(strip) }, null, 2)}\n`;

const counts = `/** Generated by \`node scripts/notices.mjs\` — how many third-party components ship. */
export const COMPONENTS = { rust: ${rust.length}, js: ${js.length} };
`;

const written = [
  [OUT, document],
  [LIST, list],
  [COUNTS, counts],
];

if (process.argv.includes("--check")) {
  const stale = written.filter(([path, want]) => read(path) !== want).map(([path]) => path);
  if (stale.length) {
    console.error(`out of date, run \`node scripts/notices.mjs\`: ${stale.join(", ")}`);
    process.exit(1);
  }
  console.log("Third-party notices are up to date.");
} else {
  mkdirSync(dirname(LIST), { recursive: true });
  for (const [path, text] of written) writeFileSync(path, text);
  console.log(`Wrote ${rust.length} crates and ${js.length} packages into ${written.length} files.`);
}

function read(path) {
  try {
    return readFileSync(path, "utf8");
  } catch {
    // Nothing generated yet, which the comparison above reports as out of date.
    return "";
  }
}
