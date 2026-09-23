# 70: A plugin brings data and shows it; it never changes what a number means

- Status: Accepted; data plugins (themes, broker layouts) are implemented, UI and compute are not yet

## Context

Two things this app will never finish by itself are broker file readers and country tax reports.
Portfolio Performance has over a hundred PDF extractors written by its community — inside its own
repository, so a new broker means a pull request and a release, and the user waits months.
Wealthfolio ships an addon SDK instead, but every addon is one kind of thing: a TypeScript page in
an iframe. Such an addon cannot be a quote source, and it cannot be a file reader, which are
exactly the two extensions this app needs most.

The seams for those two already exist and are narrow. `QuoteProvider` is `id` + `fetch` + `covers`
+ a defaulted `fetch_history` (`core/src/market/provider.rs`); `FxProvider` is smaller still. Every
reader flattens its bytes into one `ParsedCsv` and everything after that is shared
(`.claude/rules/import.md`), and since ADR-0066 there is a transaction format of the app's own for
a reader to produce. `sources::CATALOG` is already a registry of sources with capabilities and
key requirements, and `market::custom` (ADR-0054) already lets a user *describe* a source in data.
What is missing is only that all of this is compiled in.

The dangerous half is not the extension points, it is what an extension could reach. A file reader
is handed somebody's complete brokerage history; a native dynamic library handed that history can
post it anywhere, and nothing in the app could tell. And an extension that computed its own cost
basis would quietly make two users' figures incomparable, which is worse than a missing feature:
the app would no longer be able to answer "why is this number what it is".

## Decision

### One manifest, three kinds of content

A plugin is a folder with a `plugin.json` and whatever that manifest declares. Three kinds of
content, freely combined in one package — "a DEGIRO PDF reader" is a WASM module plus a broker
layout, "a dark theme" is one CSS file:

| Kind | What it is | Technology |
| --- | --- | --- |
| **Data** | broker layout, taxonomy set, theme, import keyword dictionary | JSON / CSV / CSS, no code |
| **UI** | dashboard widget, whole screen | ES module in an iframe |
| **Compute** | quote or FX source, file reader, report format, assistant tool | WASM component |

```json
{
  "id": "com.example.degiro-pdf",
  "api": 1,
  "name": "DEGIRO PDF statements",
  "version": "1.2.0",
  "provides": { "readers": ["reader.wasm"], "layouts": ["degiro.json"] },
  "needs": { "portfolio": "scope", "network": [], "secrets": [] }
}
```

`api` is the plugin API version this build speaks. A package naming another is **not loaded**, and
says so in the list — never silently, and never half-loaded.

### What is never a plugin

The model and the meaning of every figure stay compiled in:

- `TransactionKind`, lots, cost-basis methods, `Holdings` — and therefore every value built on them.
- TWR, XIRR, the risk metrics, the flow rules and the scope rewrite (ADR-0044).
- `Decimal`, the price-and-currency pairing, the FX chain (`.claude/rules/money-and-fx.md`).
- The schema and its migrations.
- The vault, profile encryption and where keys live (ADR-0048/0049).
- The consent model itself.

A plugin may bring data and draw it. If it could decide what a number means, two users could no
longer compare figures and this repository could no longer answer for either of them.

### Permissions are the assistant's, not a second vocabulary

`Free | Ask | Write` (ADR-0037) applies unchanged. `Free` is what touches no portfolio data and
asks nothing. `Ask` is a read, and a standing "always" answer covers it. **`Write` asks every
single time**, and no grant of any kind covers it.

- The manifest declares what the plugin reads, whether it writes, which **hosts** it may reach and
  which secrets it wants. The user sees that list once, at install.
- Network goes through the host against the manifest's host list. A plugin has no socket of its
  own, and a compute plugin has no network at all (below).
- Secrets are the profile's vault, under the plugin's id (`vault::Keys` is already a flat map, so
  this is a key prefix and nothing more). No plugin reads another's, and no plugin reads a key
  back — it names one, the host uses it (ADR-0048).
- A plugin that reads the portfolio sees the **scope**, not the whole portfolio, unless its
  manifest asks for the whole and the user granted it. A locked profile answers `locked` to a
  plugin exactly as it does to a command, for free: both take the store.

### Where a plugin lives

**Beside the app's settings, not inside a profile.** A profile is a folder of everything the
*portfolio* owns (ADR-0047); a theme or a file reader is not a portfolio's property, and a plugin
that vanished when the user switched profile would be a bug nobody could explain. What belongs to
the profile is what the plugin *stores*: its secrets in that profile's vault, and its state beside
that profile's database.

Trust is not solved here. Installation is a local file the user chose; there is no store, no
signature and no auto-update in this decision, and adding any of them is its own record.

### Compute is WASM, and it is a component

Component Model / WIT over wasmtime. A compute plugin gets **no filesystem, no network and no
clock** — only the arguments the host hands it. That is the whole reason it is not a native
library: a reader of somebody's brokerage statement must be *unable* to send it anywhere, and a
`.dylib` cannot offer that at any price. It also makes the language the author's business, which
matters because PDF parsing is not a Rust-shaped job, and it works where loading native code is
forbidden and JIT is banned — wasmtime's Pulley interpreter covers iOS, and parsing a file is not
a hot loop.

The two contracts come first because they are already the narrowest things in the codebase:

```
reader:      bytes + hints (file name, PDF password) -> canonical operations + warnings
quote source: id + fetch(security, range) + covers(security)
```

A reader has **no privileges**: it produces what a layout produces, and then goes through the same
preview, the same identity check and the same commit (`.claude/rules/import.md`). A plugin source
is one more row in the source catalogue, which stops being purely static.

### UI is not WASM

A UI plugin is an ES module in an iframe with an opaque origin, reaching data only through a host
bridge, with its routes declared in the manifest; the host owns the React root. A WASM frontend
buys nothing here and costs megabytes and the development loop.

## Alternatives

- **One kind of plugin, TypeScript, like Wealthfolio.** Simplest to build and rules out the two
  extensions worth having: a quote source and a file reader.
- **Native dynamic libraries.** Fast and trivially unsafe: the module gets the process, so "this
  reader cannot exfiltrate your statement" stops being something the app can promise.
- **An embedded scripting language (Lua, JS) for compute.** Sandboxed, but one language for
  everyone, and a PDF parser is exactly the case where the author's existing library decides.
- **No plugins; keep writing readers and sources in this repository.** Honest, and it is the
  Portfolio Performance outcome: contributors gated behind a release cycle, users waiting months.
- **A plugin per process instead of a sandbox.** Stronger isolation on desktop, unavailable on
  mobile, and it needs an IPC protocol of its own on top of the same contracts.
- **Installing plugins into the profile.** Reuses the vault and the folder, and makes a plugin
  disappear when the user switches profile, which is a bug in the shape of a design.

## Consequences

- `sources::CATALOG` and the reader choice in `import::parse_file` become registries that a
  plugin can extend at runtime. Both are lookups today, so this is a change of ownership rather
  than of shape.
- A new dependency, `wasmtime`, in the host — not in `core`, which keeps knowing nothing about who
  calls it.
- `UiError` gains a plugin-shaped failure: a package refused for its API version, a module that
  trapped, a network host the manifest never declared. Codes, as always, never sentences.
- A data plugin ships its conformance fixture or it does not install: the harness added for the
  shipped layouts (`core/tests/fixtures/presets/`) is what makes a stranger's layout checkable.
- The order of work follows the risk: themes (lifecycle only), broker layouts (data that already
  exists), dashboard widgets, then the file reader. The first official plugin is written by us on
  purpose — if it needs a hole in the API, the API is wrong and nobody outside has paid for it yet.
- Every plugin is a thing that can break an update. The manifest's `api` field and a visible
  "not loaded, and why" row are what keeps that from looking like the app itself failing.
