---
paths:
  - "core/src/calc/**"
  - "core/src/model/**"
  - "core/src/storage/**"
  - "core/src/money.rs"
  - "core/tests/**"
---

# Money, storage, calculation (market data: market-data.md)

- Money is `Decimal`. `f64` only for a rate/statistic (XIRR, volatility, Sharpe) or a wire format (Yahoo JSON), with a comment saying why.
- SQLite stores money as `TEXT`, dates as `TEXT 'YYYY-MM-DD'`. Conversion lives in `storage/mod.rs`, not in queries.
- Missing price/FX is `Error::MissingMarketData`, never `None`/zero. Lookups forward-fill only (as of or last before).
- A price travels with its currency (`PricePoint { close, currency }`). `Position::cost_currency` (settlement) ≠ listing quote currency.
- Minor units fold in `money::major_currency` (GBp/GBX→GBP ×0.01, ZAc, ILA, KWF ÷1000). `normalize_currency` stays a pure case fold (also used on user input).
- Trade FX rate is fixed on the transaction at conversion time (rate 1 when currency == base).
- Charges carry their currency (`fee_currency`/`tax_currency`, absent = transaction's, ADR-0064). Totals never mix currencies: `gross_in_transaction_currency` takes same-currency charges only; a foreign charge is its own cash leg (`foreign_charge_legs`) at its own pair's rate that day (`calc::holdings::charge_rate`) — `fx_rate_to_base` is never lent to another pair. `calc::holdings::charges::Charges` carries a foreign charge into the trade currency via base so lot cost stays complete.
- Buy commission goes into cost basis, never also into `Holdings::fees_base`; `Holdings::charges` = standalone Fee/Tax only. Cost *rates* count commission via `calc::costs_paid` (ADR-0024).
- Disposal beyond holdings is a ledger hole, not a short (ADR-0089): `build_holdings` refuses; `PortfolioAnalytics::transactions_until` bridges with an implied, never-stored `DeliveryInbound` at the disposal price; `quantity_gaps`/`portfolio_gaps` report it portfolio-wide. Same day: `ordered_events` applies acquisitions first (storage order is random ids).
- `Position::accounts` sums to `quantity`. A disposal from an account that never held shares goes negative there — never smeared.
- Quotes are stored split-adjusted; `corporate_actions` adjust lots, not quotes.
- Split factors aren't finite decimals: the position's product is rounded by `money::fit_quantity` (18 dp) and lots/accounts are fitted onto it, remainder on the largest part. `calc::holdings`, `calc::quantity_gaps` and the allocation breakdown must round identically.
- Period windows (ADR-0043): `period_summary` opening = `total_value_base[0] - external_flow_base[0]`, every flow counted incl. day one (zero at inception). `average_capital_base` still measures from `total_value_base[0]` (`dietz_capital` uses `date > from`). `twr_points`: no return on capital arriving the same day.
- Benchmark younger than the period: `benchmark_start` bisects, series based at that price, `Analytics::benchmark` narrows both legs to the overlap; align by date, never index. No price in the whole window is still `MissingMarketData`.
- Day iteration builds `Holdings` once via `HoldingsBuilder`. It reads the cache `PortfolioAnalytics::market_data` preloaded — whose currency set must include `fee_currency`/`tax_currency`.
- `ValueSeries` covers every calendar day; risk metrics use `business_days()` (√252). Drawdown uses chained returns.
- Price alert = a level; close ≥ level is *above*; every side change is a crossing, logged only if it matches `direction` (`calc::check_alert`). Other-currency quotes convert at their day's rate; missing rate skips the rule; no quote is `MissingMarketData`.
