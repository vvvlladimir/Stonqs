# CLAUDE.md

**Stonqs** — investment-portfolio tracker. Cargo workspace (Rust 2024, resolver 3), strictly one-way layering.

- `core/` — `sq-core`, library only: model, SQLite storage, quote/FX providers, calc. Never prints or logs; touches the filesystem only via a passed path.
- `cli/` — `sq-cli` (`sqcli`), throwaway harness for the core. No `clap`, no deps beyond core.
- `app/` — product. `app/src-tauri` = `sq-app` (Tauri host, the only crate that knows Tauri); `app/src` = Vite + React + TS.

Targets: desktop + iOS/Android from one codebase; only desktop is built, but everything is mobile-first. Browser is not a target (`libsqlite3-sys`, `ureq` don't build for wasm32).

## Commands

```bash
cargo build --workspace
cargo test --workspace                          # no network
cargo test -p sq-core --lib -- --ignored        # live ECB
cargo clippy --workspace --all-targets
cargo fmt --check                               # max_width = 110
scripts/check-rs-size.sh                        # .rs size guard
cargo test -p sq-core --test calc_examples -- twr    # one file / name filter
cargo test -p sq-core --lib calc::xirr               # one module's unit tests
cargo test -p sq-core --test phase3_import           # import suite (+ --release -- --ignored: timing)
cd fuzz && cargo +nightly fuzz run parse_any -- -max_total_time=60
cargo check -p sq-app                           # host only
cargo run -p sq-cli -- demo | series | risk | benchmark | rebalance | allocation [..] | import [file] [--resolve] [--commit]
cargo run -p sq-cli -- lookup <ISIN> | listings <ISIN> EUR | quotes AAPL <from> <to> | rates USD EUR <from> <to>   # network

cd app && pnpm install
pnpm tauri dev | pnpm dev:app (separate app.stonqs.dev data) | pnpm tauri build
pnpm build                                      # tsc --noEmit && vite build
pnpm test [--project pure|dom]                  # vitest
pnpm dev:browser                                # app in a browser, real commands over a demo (?screen=<id>)
pnpm e2e [--project webkit|webkit-ru|chromium]  # Playwright: every screen, 3 widths, axe
pnpm lint | pnpm lint:css | pnpm format
pnpm i18n:extract                               # must report no missing translations
UPDATE_FIXTURES=1 cargo test -p sq-app --test wizard_fixture   # regen e2e/fixtures/import-wizard.json
pnpm record:tour | pnpm notices | bash scripts/make-icons.sh | bash scripts/dev-signing-cert.sh
```

Dependency versions live in `[workspace.dependencies]`; members use `.workspace = true`.

## Architecture (`core/src`)

`model` knows nothing; `storage` knows `model`; `calc` knows `model` + `PriceLookup`/`RateLookup` only (no SQLite, no network).

```
model/     domain types
storage/   Store (one rusqlite Connection), repo per entity; implements PriceLookup/RateLookup
market/    QuoteProvider (yahoo/…), SecuritySearch, ListingDirectory (openfigi+mic), MarketDataService
fx/        FxProvider (ecb…), FxService — mirrors market/
inflation/ CPI indices, monthly, step lookup (ADR-0060)
sources/   catalogue of shipped sources; only builder of the services from a `Setup`
calc/      holdings/ -> valuation -> series -> risk/benchmark/allocation/rebalance/…; engine/ = PortfolioAnalytics
import/    parse_file -> parse/ | ibflex/ -> mapping/ -> preview/ -> service.rs (only Store user)
```

Host (`app/src-tauri/src`): `commands/` (thin), `state.rs`, `profiles.rs`, `vault/`+`secrets.rs`, `dbfile.rs`, `error.rs` (UiError), `plugins/`, `ai/`, `jobs/`. Frontend: `lib/api/` is the only importer of `@tauri-apps/api`.

- Order is always transactions → `Holdings` (deterministic, price-free) → `PortfolioValuation` → metrics.
- New calc code goes against `PriceLookup`/`RateLookup` (test support: `core/tests/support/mod.rs`), not `Store`.
- Providers are one-method (`fetch`). Everything synchronous — no async/tokio.

Area rules live in `.claude/rules/*.md` and load automatically by path. Changing what a screen does or what a figure means also requires updating `docs/user-guide/` or `docs/ai-reference/` in the same change (see `assistant-docs.md`).

## Conventions

- User-visible text only in the frontend, English, behind a Lingui macro (ru = catalog). Rust sends codes + values, never sentences (ADR-0023). Rust strings are English developer text.
- Comments in English, 1–2 lines, only what the code doesn't say (non-obvious decision, silent invariant, gotcha). Longer → rules file or ADR.
- Calc tests: arithmetic worked longhand in a comment first, then the code (`core/tests/calc_examples/`).
- Network tests: `#[ignore = "requires network"]`.
- Unit tests beside code; over ~100 lines → `<name>/mod.rs` + `<name>/tests.rs`. Cross-layer tests in `core/tests/`; a growing binary → `core/tests/<name>/main.rs` + modules, reaching support via `#[path = "../support/mod.rs"]`.
- Splitting a module into a folder keeps the public API byte-identical.
- Size limits (blank/comment lines don't count): Rust fn > 100 lines fails clippy unless `#[expect(clippy::too_many_lines, reason = "...")]`; `.rs` > 500 code lines warns, > 800 errors. ESLint: file 300, function 80, `.tsx` component 150; exceptions are inline `eslint-disable-next-line` with reason. Split rather than raise a limit (sub-components, `use<Thing>` hook, columns hook, `model.ts`).
- Export commonly used types in `sq_core::prelude`.

## ADRs

`docs/decisions/NNNN-kebab-title.md`. Write one for decisions on architecture, module boundaries, persistence, external integrations, concurrency, wire formats or long-lived invariants — not for refactors, bug fixes or local details (use a comment or a rules file). Format:

```markdown
# NN: Title

- Status: Proposed | Accepted | Superseded

## Context
## Decision
## Alternatives
## Consequences
```

One decision per file. Never rewrite an accepted ADR — add a new one and mark the old `Superseded by ADR-NNN`.
