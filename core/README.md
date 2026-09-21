# sq-core

The calculation and storage library behind Stonqs: domain model, local SQLite storage, quote and
FX providers, and the performance maths. It is layered so that dependencies point one way.

This is **only a library**. It knows nothing about its caller — not the desktop app, not Tauri, not
an HTTP server. It prints nothing, logs nothing, and touches the filesystem only through a path it
is handed. Exercise it with `cargo test`, or by hand through the throwaway `sq-cli` binary.

```rust
let store = Store::open("portfolio.sqlite")?;
let analytics = PortfolioAnalytics::new(&store, &portfolio);
let summary = analytics.period_summary(range)?;
```

## Layers

Dependencies point strictly one way, and that is the whole design:

```
model/      domain types. Knows nothing: not the database, not the network.
storage/    one rusqlite connection and a repository file per entity.
            Implements PriceLookup and RateLookup.
market/     quote providers, instrument search, listing directory, the service that
            caches and chains them.
fx/         the same shape for currency pairs.
inflation/  the same shape again for consumer-price indices.
import/     broker file -> parsed rows -> mapping -> preview -> one database transaction.
calc/       the maths: holdings -> valuation -> series -> every metric above it.
            Knows model plus two small traits, and nothing about SQLite or the network.
sources.rs  the catalogue of shipped data sources and the only builder of the services.
```

`calc` depending on `PriceLookup` / `RateLookup` rather than on `Store` is what makes a TWR test
run against a ten-row price table (`tests/support/mod.rs`): an SQL bug cannot hide a formula error,
and a formula test cannot be slowed down by a database.

The calculation order is always transactions → `Holdings` → `PortfolioValuation` → metrics.
`Holdings` is the deterministic replay of the ledger — quantities, lots, cost basis, realized
entries — and involves no market prices at all. Only valuation pulls a price, and every price and
rate lookup is forward-fill: as known on or before that date, never later.

Everything is synchronous. A single-user desktop app fetches a handful of tickers at a time, and
`async` would buy nothing for the cost of tokio and `Pin<Box<dyn Future>>` in every trait object.

## What comes out of it

- **TWR** answers "how did the portfolio perform", with the timing of deposits removed: the period
  is cut before each external flow and the subperiod returns are chained.
- **XIRR** answers "how much did I earn", contributions and their timing included.
- The two legitimately disagree, sometimes wildly — an annualised rate from one good month is not
  the year's return.
- Beside them: risk (volatility, drawdown, Sharpe, semi-deviation), benchmark comparison, real
  returns against a consumer-price index, allocation by classification tree / currency / account /
  instrument, rebalancing proposals in tradable units, realized gains and dividends by year and
  instrument, income and charges, investment plans, price alerts and watchlists.

## Money and missing data

Money is `rust_decimal::Decimal`, never `f64`, and is stored as SQLite `TEXT` — `REAL` is an
IEEE-754 double, which would quietly approximate every amount written to it. `f64` appears only
where the result is a *rate or a statistic* (XIRR, volatility, Sharpe) or where it is somebody
else's wire format, always with a comment saying so at that spot.

A missing price or FX rate is an error, never a silent zero: computing "somehow" means lying in a
report. A price never travels without its currency, because a listing's quote currency and a
broker's settlement currency are different things and multiplying by the wrong one is invisible.

## Data sources

| Role | Shipped |
|---|---|
| Quotes | Yahoo (default), Twelve Data, EODHD, Kraken (crypto), a source the user describes, Stooq (parser kept, live fetch blocked by a browser check) |
| FX | ECB, Frankfurter, Yahoo, a source the user describes |
| Inflation | Eurostat, IMF |
| Instruments | Yahoo search; OpenFIGI for ISIN → venue listings |

A provider has one method: fetch a range. Caching, gap-filling, chaining and rate-limit handling
live in the services above them, so no implementation repeats that logic — which is why the move
off Stooq cost one file. Quotes and FX are chains: the instrument's own source first, then the
others that cover it, with a guard that compares an overlapping window before accepting a
fallback's series.

## Broker import

Three steps, each of which can be shown to a person and overridden:

1. **Read the file.** Encoding, delimiter, header row under a preamble, a trailing total line, the
   date format and the decimal separator are all detected, and the detected values come back so the
   interface can show and change them. An Interactive Brokers Flex statement is XML and gets its
   own reader, flattened into the same shape.
2. **Map it.** Columns are matched by header name in a dozen languages — exact, then whole word,
   then substring — and the column's own *values* can veto a weak match. Operation wording is
   recognised by keyword into aliases you can edit. Rows become drafts with a status: ready,
   duplicate, unknown instrument, ignored, invalid.
3. **Commit.** One database transaction, and the only step that touches `Store`.

Detection is per *language*, never per broker: a rule keyed to one broker's export helps only that
broker's customers. Shipped broker layouts are data (`presets/brokers.json`), not code.

Re-importing the same file changes nothing — a readable fingerprint per transaction is checked
against both the database and the rows already read from this file. Duplicates are shown rather
than discarded: two genuine identical trades on one day look exactly like a repeated import, and
only a person can tell them apart.

## Tests

```bash
cargo test -p sq-core                                  # everything except network tests
cargo test -p sq-core --lib -- --ignored               # live ECB request
cargo test -p sq-core --test calc_examples             # one integration binary
cargo test -p sq-core --test calc_examples -- twr      # one test by name
cargo test -p sq-core --doc                            # the end-to-end example in lib.rs
```

Every calculation test starts with the arithmetic worked out longhand in a comment, then performs
the same computation in code. When one fails, the cause is visible without a debugger: redo the
sum on paper and see which line disagrees. Tests that need the network are `#[ignore]`d, so a clean
checkout tests green offline.

## Where the reasoning lives

- [`docs/decisions/`](../docs/decisions/) — one record per architectural decision, with what else
  was considered and what the choice costs. Start at its README.
- [`CLAUDE.md`](../CLAUDE.md) and [`.claude/rules/`](../.claude/rules/) — the working rules for each
  area: money and FX, import, migrations, the UI boundary.
- `src/lib.rs` — the layer overview and a runnable end-to-end example.
