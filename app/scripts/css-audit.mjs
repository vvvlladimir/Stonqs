// Lists every class declared in styles/ that no source file ever names, and the class count
// the UI refactor tracks. Run with `npm run css:audit` from `app/`.
import postcss from "postcss";
import fs from "fs";
import path from "path";

const files = (dir, re) => {
  const out = [];
  const walk = (d) =>
    fs.readdirSync(d, { withFileTypes: true }).forEach((e) => {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (re.test(e.name)) out.push(p);
    });
  walk(dir);
  return out;
};

const declared = new Map();
for (const f of files("src/styles", /\.css$/)) {
  postcss.parse(fs.readFileSync(f, "utf8")).walkRules((r) => {
    for (const m of r.selector.matchAll(/\.([A-Za-z][\w-]*)/g)) {
      if (!declared.has(m[1])) declared.set(m[1], []);
      declared.get(m[1]).push(`${f}:${r.source.start.line}`);
    }
  });
}

// A class counts as used only if it appears as a whitespace-delimited token of some string
// literal — `className={cond ? "row row--pick" : ""}` included, prose mentioning "bar" not.
const used = new Set();
const dynamic = new Set();
const sources = [...files("src", /\.(tsx?|html)$/), "index.html"];
for (const f of sources) {
  const text = fs.readFileSync(f, "utf8");
  const take = (body) => {
    for (const part of body.split(/\$\{[^}]*\}/)) for (const t of part.split(/\s+/)) t && used.add(t);
    // `slot-${i}` names a class only its prefix reveals.
    for (const d of body.matchAll(/([\w-]+)\$\{/g)) dynamic.add(d[1]);
  };
  // Quoted literals are scanned over the raw text too: a template placeholder often hides one
  // (`` `wgrid${editing ? " is-editing" : ""}` ``).
  for (const m of text.matchAll(/`([^`]*)`/g)) take(m[1]);
  for (const m of text.matchAll(/"([^"\n]*)"|'([^'\n]*)'/g)) take(m[1] ?? m[2] ?? "");
  for (const m of text.matchAll(/class(?:Name)?=["']([^"']*)["']/g))
    for (const t of m[1].split(/\s+/)) t && used.add(t);
}

const dead = [];
for (const [cls, where] of declared) {
  if (used.has(cls)) continue;
  if ([...dynamic].some((p) => cls.startsWith(p))) continue;
  dead.push(`${cls}\t${where.join(" ")}`);
}
console.log(dead.sort().join("\n"));
console.log("declared:", declared.size, "| unused:", dead.length);
