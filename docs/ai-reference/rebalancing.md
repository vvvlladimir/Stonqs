# Rebalancing

A rebalance compares a classification tree against target weights and proposes trades. It proposes
only: nothing is written until the user acts on it.

**A target weight is a share of its parent, not of the portfolio.** Equities 60%, and inside
equities Europe 30%, means Europe is 18% of the portfolio. Absolute shares are always derived by
multiplying along the path, and sibling targets are validated as a set.

Only the deepest weighted nodes divide money. A weighted parent gets its own row and its own drift,
but it does not produce trades of its own — its children do.

Two options change the arithmetic, not the display:

- **Cash to invest** is added to the total *before* targets are applied, so the plan spends new
  money rather than assuming it is already invested.
- **Do not sell** leaves overweight nodes alone and splits the new money across the underweight
  ones in proportion to their size. Shortfalls almost always exceed the cash available, so this is
  the common case: the result reduces drift, it does not remove it.

New money is spent down to whole tradable units, largest shortfall first. Whatever is left over is
smaller than the cheapest unit that could be bought and is reported as remaining cash, not hidden.

Cash is a subject like any other, so a node whose target is cash is met by paying money in, not by
buying something.
