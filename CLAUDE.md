# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Cargo workspace (Rust 2024, resolver 3) for **Stonqs**, an investment-portfolio tracker.
Layering is strictly one-way (see Architecture).

- `core/` — `sq-core`, a **library only**: model, SQLite storage, quote/FX providers, performance math. It never prints, never logs, and touches the filesystem only via an explicitly passed path.
- `cli/` — `sq-cli` (binary `sqcli`), a throwaway harness for exercising the core by hand. Not a product. Deliberately has no `clap` and no dependencies beyond the core.
- `app/` — the product UI. `app/src-tauri` is `sq-app`, the Tauri host (Rust); `app/src` is the frontend (Vite + React + TypeScript). This is the only crate that knows Tauri exists.

Target platforms are macOS/Windows/Linux **and** iOS/iPadOS/Android from the same code. Only desktop is built for now, deliberately. Everything is nonetheless written mobile-first — see `.claude/rules/ui-boundary.md`. The browser is explicitly not a target: `libsqlite3-sys` and `ureq` do not exist on `wasm32-unknown-unknown`.

## Commands

```bash
cargo build --workspace
cargo test --workspace                             # unit + integration, no network
cargo test -p sq-core --lib -- --ignored    # live ECB request (fx/ecb.rs)
cargo clippy --workspace --all-targets
cargo fmt --check                                  # rustfmt.toml sets max_width = 110

cargo test -p sq-core --test calc_examples          # one integration file
cargo test -p sq-core --test calc_examples -- twr   # one test by name substring
cargo test -p sq-core --lib calc::xirr              # unit tests of one module
cargo test -p sq-core --doc                         # lib.rs doctest (end-to-end example)

cargo run -p sq-cli -- demo                            # in-memory end-to-end scenario
cargo run -p sq-cli -- series | risk | benchmark | rebalance
cargo run -p sq-cli -- allocation [taxonomy|currency|account|security]
cargo run -p sq-cli -- import [file] [--resolve] [--commit]  # CSV preview / write
cargo run -p sq-cli -- lookup IE00B5BMR087             # ISIN -> ticker (network)
cargo run -p sq-cli -- listings IE00B4L5Y983 EUR       # ISIN -> venues (network)
cargo run -p sq-cli -- import prices [file] [symbol] [--commit]
cargo run -p sq-cli -- quotes AAPL 2024-06-03 2024-06-10   # needs network
cargo run -p sq-cli -- rates USD EUR 2024-06-03 2024-06-08 # needs network
```

```bash
cd app
pnpm install
pnpm tauri dev           # desktop app against the release data dir (app.stonqs)
pnpm dev:app             # same, but identifier app.stonqs.dev -> its own profiles/DB
pnpm tauri build         # bundled desktop app
pnpm build               # frontend only: tsc --noEmit && vite build
pnpm record:tour         # fresh demo profile, every screen, IPC -> e2e/fixtures/ipc.json (for site screenshots)
pnpm i18n:extract        # refresh src/locales/{en,ru}/messages.po from the code
pnpm i18n:compile        # compile catalogs (the Vite plugin does this during a build)
bash scripts/make-icons.sh    # regenerate every icon from app-icon.svg
cd ../site && pnpm screenshots [--only a,b] [--preview]  # site images + og.jpg from that fixture
bash scripts/dev-signing-cert.sh  # once per Mac: stable dev signature, so keychain "Always Allow" sticks
```

`cargo check -p sq-app` type-checks the host without touching the frontend.
Dependency versions live in `[workspace.dependencies]`; member crates use `.workspace = true`.

## Architecture

Dependencies point strictly one way: `model` knows nothing; `storage` knows `model`; `calc` knows `model` plus two small traits, and knows nothing about SQLite or the network.

```
model/    domain types (Account, AccountGroup, Security, SecurityAttributeDef, Transaction,
          Position+Lot, Portfolio, CorporateAction, Taxonomy, CostBasisMethod, InvestmentPlan,
          SecurityAlert, SecurityEvent, Watchlist)
storage/  Store owns one rusqlite Connection; per-entity repository files.
          Implements PriceLookup (quotes.rs) and RateLookup (fx_rates.rs).
market/   QuoteProvider (yahoo.rs live, stooq.rs kept as a second example),
          SecuritySearch (instrument directory), ListingDirectory (openfigi.rs +
          mic.rs), MarketDataService = provider registry + cache + resolve().
fx/       mirrors market/: FxProvider, ecb.rs, StaticFxProvider (offline/tests), FxService.
inflation/ mirrors fx/ again for consumer-price indices (eurostat.rs, imf.rs, codes.rs) — an
          index is monthly, so its lookup steps rather than interpolates (ADR-0060).
sources.rs the catalogue of shipped sources (SourceInfo rows, one constructor per role) and
          the only builder of MarketDataService/FxService from a `Setup` (keys, switches,
          custom sources — ADR-0050/0053). Quotes and FX are chains with a fallback guard
          (market/guard.rs, ADR-0051/0052); market/custom.rs is the user-described feed (ADR-0054).
calc/     holdings.rs -> valuation.rs -> series.rs -> risk.rs / benchmark.rs
          -> allocation.rs / rebalance/, capital_gains.rs, dividends.rs,
          income.rs, charges.rs, journal.rs, periods.rs, plans.rs, alerts.rs, watchlist.rs;
          twr.rs / xirr.rs;
          engine/ glues everything as PortfolioAnalytics, its impl split by subject.
app/      Tauri host: commands/ (thin), state.rs (Mutex<Store> + Portfolio of the open
          profile), profiles.rs (one folder per profile), vault.rs + secrets.rs
          (password-sealed provider keys, the lock), dbfile.rs (plain <-> encrypted
          database file, ADR-0049),
          error.rs (UiError). Frontend: lib/api.ts is the only file that
          imports @tauri-apps/api; lib/types/ is a barrel, still imported as
          "lib/types".
import/   parse_file -> parse.rs | ibflex.rs (IB Flex XML, ADR-0061) -> mapping/
          -> preview/ -> service.rs (only place that touches Store).
          taxonomy.rs handles taxonomy CSV. checks.rs is the
          plausibility layer. presets.rs ships broker layouts from
          presets/brokers.json.
```

The calculation order is always transactions → `Holdings` → `PortfolioValuation` → metrics. `Holdings` is deterministic and price-free; only valuation pulls market data.

`PriceLookup` / `RateLookup` are the seam that keeps `calc` testable (`core/tests/support/mod.rs`). Add new calc code against these traits, not against `Store`.

Providers stay one-method (`fetch`; `fetch_history` is a default overridden only when that one response also carries dividends and splits). Everything is synchronous — no async, no tokio (single-user desktop app).

## Invariants

Detailed, per-area rules live in `.claude/rules/` and are loaded only when relevant — keep this file itself short:

- `.claude/rules/money-and-fx.md` — Decimal vs f64, storage encoding, price/currency pairing, FX rates, quote/fx caching.
- `.claude/rules/taxonomy-and-rebalance.md` — allocation subjects, exclusions, target weights, rebalance math.
- `.claude/rules/import.md` — which reader a file gets, CSV parsing, amount-sign detection, row severity, taxonomy import.
- `.claude/rules/ui-boundary.md` — sq-app ↔ sq-core boundary, wire format, scoping.
- `.claude/rules/ai-assistant.md` — the assistant in `app/src-tauri/src/ai/`: layering, tool catalogue, consent, the dashboard brief.
- `.claude/rules/frontend.md` — layers inside `app/src`: queries, components, primitives, dashboard, i18n.
- `.claude/rules/migrations.md` — migration history and rules.
- `.claude/rules/assistant-docs.md` — `docs/user-guide/` and `docs/ai-reference/`: which corpus, when a screen change obliges a guide change, how to write one, how to register it.

When editing code in one of these areas, read the matching rule file first. Changing what a screen
*does*, or what a figure *means*, also obliges the matching file under `docs/user-guide/` or
`docs/ai-reference/` in the same change — that corpus is what the AI assistant answers from, so a
stale one makes it confidently wrong rather than merely vague (`.claude/rules/assistant-docs.md`).

## Conventions

- User-visible text lives in the frontend only, in English, behind a Lingui macro; Russian is a
  catalog (`app/src/locales/ru`). Rust sends codes and values, never sentences — see ADR-0023 and
  `.claude/rules/ui-boundary.md`. Strings that remain in `core`, `app/src-tauri` and `cli` are
  English developer text.
- Doc comments and code comments are written in English. Comment only what the code does not already say: a non-obvious decision, an invariant that would break silently, or a gotcha a future reader would hit. Skip comments that restate the line below them. Keep comments to 1–2 lines; move longer explanations to the relevant `.claude/rules/*.md` file or an ADR.
- Every calculation test starts with the arithmetic worked out longhand in a comment, then the same computation in code — see `core/tests/calc_examples/`. Keep that shape.
- Tests needing network are `#[ignore = "requires network"]`.
- Unit tests live next to the code (`mod tests`); cross-layer tests live in `core/tests/`. A test
  binary that outgrows one file becomes `core/tests/<name>/main.rs` plus a module per subject —
  one binary, so link time does not grow with the number of themes. `support` is shared, so a
  folder reaches it with `#[path = "../support/mod.rs"]`.
- A module that outgrows one file becomes a folder with `mod.rs`, and the public API stays byte for
  byte what it was: callers keep importing `import::mapping`, not `import::mapping::aliases`.
- `sq_core::prelude` exists for callers touching many modules; export new commonly used types there.

## Architecture Decision Records

Store ADRs in `docs/decisions/` as Markdown files. Use a sortable numeric prefix
and a short kebab-case title, for example `0013-provider-cache-policy.md`.

Create an ADR when a decision affects architecture, module boundaries, persistence,
external integrations, concurrency, wire formats, or a long-lived invariant; record
the context, the chosen option, alternatives considered, and consequences. Do not
create an ADR for routine implementation details, local refactors, bug fixes, or
temporary experiments. Use a nearby code comment for a small local invariant and a
`.claude/rules/*.md` file for a reusable operational rule.

Use this structure:

```markdown
# NN: Decision title

- Status: Proposed | Accepted | Superseded

## Context
## Decision
## Alternatives
## Consequences
```

Keep one decision per file. Never silently rewrite an accepted decision: if the
decision changes, add a new ADR and mark the old one `Superseded by ADR-NNN`.
Reference the ADR from code only when the local comment needs its rationale.