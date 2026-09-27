# 81: A trade may be cut per lot

- Status: Accepted

## Context

ADR-0027 builds a trade from the lots a disposal consumed: one trade per disposal, one per open
position. That answers "how did holding this go". Portfolio Performance also offers the other
question — "how did each purchase go" — where every buy is its own trade and a sale is divided
over the purchases it touched. A saver who buys monthly sees one open trade under ADR-0027 and
twelve under the other reading, and both are true.

## Decision

- `calc::TradeGrouping { Position, Lot }`, `Position` the default. `closed_trades` and
  `open_trades` take it; `Position` is exactly ADR-0027.
- `Lot` makes one trade per lot from the same lots — no new calculation. A disposal's net
  proceeds, and an open position's market value, are shared between its lots **by quantity**;
  the last lot takes what the division left, so the parts add up to the whole to the last digit
  and the per-lot results sum to the per-position result.
- A lot partly consumed by a sale is a closed trade for the consumed part and stays in the open
  trade for the rest — that is what FIFO lots already hold.
- The Trades screen offers the switch and remembers it in `UiState::trades_by`. Dashboard tiles
  keep `Position`; the assistant's `report_trades` takes an optional `by`.

## Alternatives

- **Share the proceeds by cost instead of quantity.** A sale gets one price per share; dividing
  it by what each lot cost would pay the dearer lot more for the same share.
- **Replace the per-position reading.** It answers a different question and the dashboard's
  counts are built on it.

## Consequences

- Under average cost a position has a single merged lot, so both groupings give the same trades.
- The win rate and the average holding period differ between the two readings, because the count
  of trades does; the total result does not.
