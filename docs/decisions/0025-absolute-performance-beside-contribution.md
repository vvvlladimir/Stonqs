# 25: Absolute performance divides by the position's own capital

- Status: Accepted

## Context

ADR-0021 settled that `PositionReturnRow::contribution` divides a position's P/L by the
*portfolio's* capital at work, so the column sums to the portfolio's Modified Dietz return.
That property is what makes the contribution waterfall reconcile.

A securities table can also carry an "Absolute Performance %". It answers a
question the contribution column cannot: not "how much of the portfolio's result came from
here" but "what did the money in this instrument earn". A holding worth 2% of the portfolio
that doubled contributes about two points and performed one hundred percent; only the second
number tells the owner whether to buy more of it.

TWR does not replace it either. TWR chains sub-period returns and is therefore free of when
money was added — the right answer for comparing instruments, the wrong one for "what did I
make on the money I actually had in".

## Decision

`PositionReturnRow` gains `absolute_performance`: `pnl_base` divided by the Modified Dietz
denominator computed over the *position's own* flows — its purchases, sales and dividends —
rather than the portfolio's. `None` when the position had no capital at work, never zero.

Both denominators come from one helper, `calc::series::dietz_capital`, which takes a starting
value and an iterator of dated flows. `position_returns` calls it twice per period: once for
the portfolio, once per position.

## Alternatives

- **P/L over purchase value.** What PP's moving-average column effectively is, and simpler,
  but it ignores when the money went in: a position doubled in size on the last day of the
  period would dilute its own return.
- **Drop contribution and keep only this.** The waterfall would stop reconciling to the
  portfolio return, which is the one property that makes it trustworthy.

## Consequences

- Three return columns sit side by side and disagree, by design: `twr` (time-weighted, free of
  flow timing), `absolute_performance` (money-weighted, own capital), `contribution`
  (money-weighted, portfolio capital). Each column's tooltip names its denominator.
- `absolute_performance` does not sum to anything across positions, and must not be presented
  as if it did.
- The private `average_capital` in `engine.rs` is gone; both call sites now share
  `dietz_capital`, so a change to the weighting rule can no longer apply to one and not the
  other.
