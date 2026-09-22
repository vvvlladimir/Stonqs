# Money, storage, FX

- Money is `Decimal`, never `f64`. `f64` only where a *rate or statistic* is computed (XIRR, volatility, Sharpe) or where it's a wire format (Yahoo JSON) — always with a comment saying why at that use site.
- Money is stored in SQLite as `TEXT` (SQLite `REAL` is a double); dates as `TEXT 'YYYY-MM-DD'` so lexicographic order equals chronological order. Conversion lives in `storage/mod.rs`, not individual queries.
- Missing price or FX rate is `Error::MissingMarketData`, never a silent `None`/zero.
- A price never travels without its currency (`PriceLookup` returns `PricePoint { close, currency }`). `Position::cost_currency` (settlement currency) is a different thing from the listing's quote currency — conflating them multiplies a dollar price by EUR/EUR.
- Minor currency units fold into major ones in `money::major_currency` (GBp/GBX → GBP ×0.01, ZAc → ZAR, ILA → ILS, KWF → KWD ÷1000). `normalize_currency` stays a pure case fold — it's also called on user input.
- Lookups are forward-fill only ("as of, or last known before"), never forward in time.
- A window opens on what was there *before* it: `period_summary` takes `total_value_base[0] - external_flow_base[0]` as the opening balance and counts **every** flow, the first day's included. At inception that opening is zero rather than the first purchase. `average_capital_base` still measures from `total_value_base[0]` — money that arrived on the opening day worked the whole window, which is what `dietz_capital`'s `date > from` filter already assumes — so `delta_base`, `invested_capital_base` and every cost rate are unmoved. `twr_points` keeps its own rule: a return cannot be earned on capital that arrived the same day. See ADR-0043.
- A benchmark younger than the period is not missing data: `benchmark_start` bisects the day
  (forward fill makes "has a price" monotone), `benchmark_series` begins there and is based at
  that price, and `Analytics::benchmark` narrows *both* legs to that overlap so `excess` compares
  equal windows — `BenchmarkComparison::from` reports the window compared, not the one asked for.
  The two series are aligned by date, never by index. No price anywhere in the window is still
  `MissingMarketData`.
- FX is a chain (`FxService::ensure_rates`, ADR-0051): `sources::FX_ORDER`: ECB → Frankfurter (ECB mirror) → Yahoo `BASEQUOTE=X`; the pair's first covering source upserts, a later one only fills (`save_fx_rates_from(.., false)`). A source is skipped for a pair when `FxProvider::covers` says it does not publish one of the currencies; an **error** falls through to the next, an **empty** answer is final — a weekend must not pull a second source into the series. `ecb.rs::PUBLISHED` is today's ECB list, so RUB, ARS etc. go straight to the market source.
- Inverse rates = `1 / rate`. Cross-rates are synthesized only inside `EcbProvider` (both legs from one source, one date) — not during DB lookup.
- The trade's FX rate is fixed on the transaction at conversion time, not rewritten when rates move later. Doesn't apply when transaction currency == base currency (rate is 1 by definition).
- A charge carries the currency it was billed in (`Transaction::fee_currency` / `tax_currency`,
  absent = the transaction's own, ADR-0064). A total never mixes currencies:
  `gross_in_transaction_currency` folds in only the charges in that currency, a foreign one is a
  cash movement of its own (`foreign_charge_legs`) and is converted at *its* pair's rate on the
  transaction's date (`calc::holdings::charge_rate`) — `fx_rate_to_base` belongs to the
  transaction's currency and is never lent to another pair. `calc::holdings::Charges` carries a
  foreign charge back into the trade's currency through the base one so the lot's cost stays
  complete; both rates are of the same day, which is arithmetic over what was paid, not a cross
  rate synthesized at lookup.
- Buy commission goes into cost basis; never also counted in `Holdings::fees_base`. `Holdings::charges` records only standalone Fee/Tax operations. A cost *rate* asks the opposite question and must count that commission, so it goes through `calc::costs_paid`, never through the charges rollups — see ADR-0024.
- `Position::accounts` values sum to `quantity`. A disposal from an account that never held the shares goes negative there rather than being smeared over other accounts — that's a data inconsistency, and hiding it forges the answer to "where is it".
- Quotes are stored already split-adjusted; `corporate_actions` adjust lots, not quotes.
- `quote_coverage` records what was asked; `quotes` records what came back. Extend coverage even when a provider returns nothing.
- Anything iterating over days builds `Holdings` once via `HoldingsBuilder`, not `holdings_at` per day.
- `ValueSeries` covers every calendar day; risk metrics run on `business_days()` (√252 annualisation assumes trading days). Drawdown uses chained returns, not portfolio value.
- Which sources exist is `sources::CATALOG`, nothing else: a provider's id is its `ID` constant, services are built by `sources::quote_service()` / `fx_service()`, and a call site that needs "the" source names `sources::DEFAULT_QUOTES` / `DEFAULT_FX`, never a string literal. A 429 is `Error::RateLimited` (transient), a 401/403 `Error::Unauthorized` (never retried) — see ADR-0050.
- Quotes are a chain too (ADR-0052): the instrument's own source first; only on its **failure** the others that `covers` it and have its symbol in `security_symbols`, asked 14 days early so `market::guard::check` can compare (same currency, median ratio within 2%). An accepted fallback `fill_quotes` and extends no coverage. A source failing 3× in a row (or once with a rejected key) rests for the rest of the service's life. Kraken covers `Crypto` only; Stooq is off (JS challenge). A source with `SourceInfo::per_day` spends from `Budgets` before each call and is `RateLimited` once the day is spent (ADR-0055). A custom source (`market::custom`, ADR-0054) is quotes or FX by `CustomRole`; a rate one joins the FX chain last.
- Providers are one-method (`fetch(&Security, DateRange)`). Caching, gap-filling, retries belong in `MarketDataService`/`FxService`, never in a provider. `fetch_history` is a default method a provider overrides only when the same response also carries dividends and splits (Yahoo's `events=div|split`) — never a second request (ADR-0034).
- A range counts as fetched only when both `quote_coverage` and `event_coverage` hold it; the refresh job asks the full window for a security with no event coverage, so older databases backfill events once. A reported event moves no money and no quantity.
- How far back a refresh reaches is read off the ledger, not off a constant: `Store::history_need`
  gives the first operation per instrument and per currency (charge currencies and the security's
  own currency included), and `jobs::start_from` widens the mode's window back to it whenever what
  is stored does not already reach it. A catch-up asks from where the series ends, so a hole older
  than the series is one no later refresh would ever close — which is what importing a decade into
  a database holding this year looks like. `RefreshMode::Missing` therefore also *retains* what is
  covered but not covered far enough.
- A source that answers with today's close alone is on the wrong venue, and that is not an error
  the provider reports: `Store::quote_span` (what came back) against `quote_coverage` (what was
  asked) is the only place the two diverge. `jobs::sparse_history` calls a series covering under a
  quarter of the holding period too short — never below 90 days of holding, so a young position
  says nothing — and `jobs::relist` then moves the instrument to `best_listing` /
  `best_listing_by_symbol` and fetches it there. **Only in `RefreshMode::Missing`**: a venue the
  user chose by hand is never overwritten by a refresh they pressed. A relisted instrument emits
  `securities`, not only `quotes` — its ticker and currency changed with the series.
- A new instrument gets its quotes at once, whatever wrote it: every command that creates or re-points a security or writes transactions (`security_save`, `security_identify`, `security_set_listing`, `import_commit`, `transaction_save`, `plan_commit`) calls `jobs::fetch_missing`. That `Missing` refresh fetches only securities lacking either coverage and currency pairs with no rate, full window, and is queued behind a running refresh — which listed its instruments before the new one existed. The frontend never starts it. A security with no `data_source` is manual prices and stays untouched; a new one from the form defaults to a provider.
- A listing is chosen, not guessed: one ISIN maps to many venue tickers (IWDA.L vs EUNL.DE vs SWDA). `ListingDirectory` (OpenFIGI) gives MIC + ticker, `SecuritySearch::symbol_for` builds the provider symbol, `best_listing` prefers base currency — final say is the user's. Switching a listing deletes that security's quotes and `quote_coverage`, and the events its provider reported with `event_coverage`; the user's notes stay. Choosing the listing whose provider symbol is already set deletes nothing — the series belongs to the symbol, and naming its venue is a correction, so `security_set_listing` compares `provider_symbol()` first.
- Reading the venue off the source is not guessing it: `SecuritySearch::mic_for` (inverse of
  `symbol_for`) says which listing the price series came from — Yahoo from its suffix table, falling
  back to its own exchange names only for the suffix-less US venues, where one ticker is shared by
  XNAS/XNYS/ARCX. An unknown suffix is an unsupported venue, not a US listing. `SecurityMatch` and
  `SecurityDraft` carry that `mic`, so `security_save`, `security_identify` and the import all record
  it; `security_identify` writes what the source named rather than clearing it. See ADR-0036.
- Yahoo returns no ISIN anywhere, and nothing free maps a ticker to one. An instrument without an
  ISIN is therefore normal, and the venue picker still works for it: `listings_by_symbol` searches
  the bare ticker and keeps the matches on that same ticker that the source can place on a supported
  venue. `Listing::isin` is empty there and the answer is not cached — the listings table is keyed by
  ISIN.
- A price alert is a level: a close at or above it is *above*, and every change of side between two closes is a crossing — logged, with the level and close of that moment, only when it matches the rule's `direction` (`UP | DOWN | BOTH`) (`calc::check_alert`). A quote in another currency is converted at the rate of its own day; a missing rate skips the rule until it arrives, and a status with no quote is `MissingMarketData`, never "far from the level".
- `resolve` doesn't trust search ranking — some results carry a name/currency but no candles. Probe with `profile` until `has_history == Some(true)`; a preferred currency breaks ties.
- An ISIN is never a provider symbol (`Security::is_quotable()` is false for one). `MarketDataService` refuses before the request. Fix via `security_identify`.
- Resolving a broker code to a real instrument is network, so it never happens inside `build_preview` (must stay reproducible) — the wizard does it via `import_resolve_symbol`, which the Instruments step starts by itself on arrival. Search first, then the **directory**: a code the search places nowhere still has venues, and `best_listing` / `best_listing_by_symbol` probe each for candles before one is taken (`SecurityDraft::from_listing`). A listing carries no kind, so such a draft is `Other`.
- `Security::mic` survives the listing choice; the venue name is never stored, only derived via `market::mic` (closed ISO 10383 list).