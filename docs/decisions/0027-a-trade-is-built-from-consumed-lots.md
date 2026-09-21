# 27: A trade is built from the lots a disposal consumed

- Status: Accepted

## Context

Other trackers treat a trade as an object of its own: an open trade begins with the
first purchase and grows with later ones, and every sale produces a closed trade carrying an
entry value, an exit value, a holding period and an IRR. Stonqs had the ingredients — FIFO
lots in `Position`, one `RealizedGain` per disposal — but not the object: `RealizedGain`
recorded only the *total* base-currency cost of what it removed. A holding period weighted by
quantity and an IRR from irregular dates both need the individual purchase dates, and those
were dropped the moment the lots were consumed.

## Decision

`RealizedGain` keeps the lots it consumed (`lots: Vec<Lot>`), alongside their cost in the
settlement currency. `calc/trades.rs` is then pure aggregation over `Holdings`, with no
transaction walk of its own:

- `closed_trades(&holdings.realized)` — one trade per disposal, exit value = proceeds net of
  the sale's own fees and taxes.
- `open_trades(&holdings, &valuation)` — one trade per open position, exit value = market value.
- `trade_stats(&[Trade])` — counts, result, win rate, and a holding period weighted by entry
  value.
- `trading_volume(...)` — purchases plus sales over a period: the numerator of a turnover rate,
  whose denominator is `PeriodSummary::average_capital_base`.

A trade's holding period is weighted by quantity, because one trade may be built from purchases
of different ages. Its IRR uses each lot's own date as a negative flow and the exit as the
positive one, so it is comparable with `position_xirr`.

An outbound delivery closes a trade like a sale does. It realizes a result, and the shares'
whole history would otherwise vanish from the ledger — the same rule `capital_gains` already
follows.

Dividends are not part of a trade. A trade measures the decision to hold the shares between two
dates; payments received while holding them are `income.rs`, and mixing them would make the
trade's return disagree with `return_on_cost` on the same disposal.

## Alternatives

- **A separate FIFO pass inside `trades.rs`.** Rejected: it duplicates `holdings.rs`'s lot
  matching, and two implementations of FIFO in one crate will disagree the first time a
  corporate action or an average-cost portfolio touches them.
- **Recording only the oldest lot's date.** Enough for a holding period, useless for an IRR,
  and wrong as soon as one sale spans purchases years apart.
- **A trade as a stored entity with its own table.** Rejected: a trade is derived, like
  `Holdings` itself. Storing it would mean invalidating it on every edited transaction.

## Consequences

- `Holdings` grows by one `Lot` vector per disposal. It is derived state, never persisted, and
  a disposal consumes a handful of lots at most.
- `RealizedGain` gains fields on the wire; both are `#[serde(default)]`, so an older payload
  still deserializes with an empty lot list and a zero currency gain.
- Under `CostBasisMethod::AverageCost` a position holds one merged lot, so its trade opens at
  the earliest purchase in that lot and has a single age. That is the method's own answer, not
  a loss of detail in trades.
