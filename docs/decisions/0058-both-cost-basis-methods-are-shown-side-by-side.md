# 58: Both cost-basis methods are shown side by side, and neither becomes the portfolio's

- Status: Accepted

## Context

The portfolio has one cost-basis method (`CostBasisMethod`, FIFO or average cost) and every figure
in `calc` uses it. Some trackers instead print purchase value and purchase price twice —
once FIFO, once moving average — because the two are tax conventions rather than valuations, and a
user cannot compare what they cannot see.

The methods disagree about exactly one thing: which shares a sale consumed. Quantity, price and
market value are method-free, and so is everything read off the value series (TWR, volatility,
drawdown) — none of it touches a cost.

## Decision

- `calc::compare_cost_basis(fifo, average, valuation)` is pure and takes **two holdings and one
  valuation**: the passes differ only in `HoldingsOptions::cost_basis`, so quantities are identical
  and no price is read twice. `PortfolioAnalytics::cost_basis_comparison` is the glue that builds
  the two passes.
- The portfolio's own method is **unchanged and unqueried** here. The comparison is a reading, not
  a second setting, and no calculation elsewhere switches method.
- The wire carries one object per method (`fifo`, `average`) of identical shape, never prefixed
  fields, so a column is a lookup and the two stay comparable. The response does **not** say which
  method the portfolio uses: a column whose meaning moved with a setting could not be read beside
  the one next to it.
- `positions_cost_basis` is its own command, and its query stays idle until one of its columns is
  switched on — it costs a second holdings pass over the whole ledger.
- Only **open** positions are listed. A closed position has no purchase value left to disagree
  about, and its realised total is the same either way.

## Alternatives

- **More fields on `positions_at`.** Every positions query, on every screen and widget, would pay
  for a second holdings pass that most of them never display.
- **Send only "the other method".** Half the payload, but the column's meaning would depend on a
  setting, and two users' screenshots of the same column would mean different things.
- **A toggle switching the whole screen between methods.** Answers "what would it look like",
  never "how far apart are they", which is the actual question — and invites reading a realised
  figure under a method the tax return does not use.

## Consequences

- A partly sold position is the only row where the pair differs; everywhere else the two columns
  agree, which is itself the useful answer.
- Realised plus unrealised is equal under both methods for any position, so the pair can be
  checked against each other rather than trusted.
- Changing the portfolio's method still changes every other figure in the app; this only adds a
  way to see what that change would be worth before making it.
