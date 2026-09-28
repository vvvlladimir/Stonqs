---
paths:
  - "core/src/market/**"
  - "core/src/fx/**"
  - "core/src/inflation/**"
  - "core/src/sources/**"
  - "core/src/storage/quotes.rs"
  - "core/src/storage/fx_rates.rs"
  - "core/src/storage/listings.rs"
  - "core/src/storage/securities.rs"
  - "app/src-tauri/src/jobs/**"
  - "app/src-tauri/src/commands/sources.rs"
  - "app/src-tauri/src/commands/listings.rs"
  - "app/src-tauri/src/commands/lookup.rs"
  - "app/src-tauri/src/commands/securities.rs"
---

# Market data: sources, quotes, FX, listings

## Sources
- `sources::CATALOG` is the only list. IDs are the provider's `ID` const; services are built by `sources::quote_service()`/`fx_service()`; call sites use `sources::DEFAULT_FX`/`DEFAULT_INDEX`, never literals. 429 → `Error::RateLimited` (transient), 401/403 → `Error::Unauthorized` (never retried) (ADR-0050).
- No default quote source (ADR-0076): `sources::default_quotes(&Setup)` = first source switched on, else `None` (manual prices). While `AppSettings::sources_configured` is false, `AppState::market_setup` turns every switch off and `jobs::start` refuses.
- Providers are one-method (`fetch(&Security, DateRange)`); caching/gap-filling/retries live in the services. `fetch_history` overridden only when the same response carries dividends/splits — never a second request (ADR-0034).
- Keyed sources with `SourceInfo::per_day` spend from `Budgets` before each call (ADR-0055). Custom sources: `market::custom`, `CustomRole` quotes or FX; FX joins the chain last (ADR-0054). Kraken covers `Crypto` only (not on by default); Stooq is off.

## Quote chain (ADR-0052)
- Own source first; on **failure only**, others that `covers` it and have its symbol in `security_symbols`, asked 14 days early so `market::guard::check` compares (same currency, median ratio within 2%). Accepted fallback → `fill_quotes`, no coverage. 3 failures in a row (or one rejected key) → source rests for the service's life.
- Latest close (`security_symbols.latest`, ≤1 per instrument, ADR-0079): `ensure_latest` after history, only when the series ends before today, writes only later days, guarded like a fallback.
- `quote_coverage` = what was asked, `quotes` = what came back; extend coverage even on empty answers. A range is fetched only when both `quote_coverage` and `event_coverage` hold it. Reported events move no money/quantity.
- Refresh reach comes from the ledger: `Store::history_need` (first op per instrument and currency, charge currencies incl.); `jobs::window::start_from` widens back to it. `RefreshMode::Missing` also retains covered-but-too-short ranges.
- Wrong-venue detection: `Store::quote_span` vs `quote_coverage`; `jobs::sparse_history` (< quarter of holding period, never below 90 days held) → `jobs::relist` to `best_listing(_by_symbol)`. **Only in `RefreshMode::Missing`**; emits `securities`, not just `quotes`.
- Every command creating/re-pointing a security or writing transactions (`security_save`, `security_identify`, `security_set_listing`, `import_commit`, `transaction_save`, `plan_commit`) calls `jobs::fetch_missing` (queued behind a running refresh). Frontend never starts it. No `data_source` = manual prices, untouched; `securities_adopt_source` offers one once.

## FX (ADR-0051)
- Chain `sources::FX_ORDER`: ECB → Frankfurter → Yahoo `BASEQUOTE=X`. First covering source upserts; later ones only fill (`save_fx_rates_from(.., false)`). Skip when `FxProvider::covers` says no; **error** falls through, **empty** answer is final (weekends). `ecb.rs::PUBLISHED` is today's ECB list.
- Inverse = `1 / rate`. Cross rates only inside `EcbProvider` (same source, same date), never at lookup.

## Listings and identity
- One ISIN → many venues. `ListingDirectory` (OpenFIGI) gives MIC + ticker, `SecuritySearch::symbol_for` builds the symbol, `best_listing` prefers base currency; user has final say. Switching listing deletes quotes, `quote_coverage`, provider events and `event_coverage` (user notes stay) — unless `provider_symbol()` is unchanged (`security_set_listing` compares first).
- `SecuritySearch::mic_for` reads the venue off the symbol (Yahoo suffix table; exchange names only for suffix-less US). Unknown suffix = unsupported, not US. `SecurityMatch`/`SecurityDraft` carry `mic` (ADR-0036). `Security::mic` stored; venue name derived via `market::mic` only.
- Yahoo has no ISINs; instruments without one are normal. `listings_by_symbol` searches the bare ticker; `Listing::isin` empty, not cached.
- `resolve` probes `profile` until `has_history == Some(true)`; preferred currency breaks ties.
- An ISIN is never a provider symbol (`Security::is_quotable()` false); service refuses before the request. Fix via `security_identify`.
- Resolving broker codes is network → never in `build_preview`; the wizard calls `import_resolve_symbol`: search, then directory, `best_listing*` probes candles (`SecurityDraft::from_listing`, kind `Other`).
