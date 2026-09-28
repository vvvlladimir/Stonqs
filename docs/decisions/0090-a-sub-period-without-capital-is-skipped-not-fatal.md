# 90: A sub-period the ledger cannot account for is skipped, not fatal

- Status: Accepted

## Context

TWR chains one factor per sub-period, `V_i / (V_{i-1} + F_i)`. The denominator is what the
portfolio had at work; a first import often makes it meaningless, because a broker file is an
export of *trades*, not of a bank account. Buying without a deposit behind it leaves the cash
account below zero, and a dividends-only file makes value appear with no flow to explain it.

Both were read as arithmetic failures. Value from zero capital returned `Error::Math`, and
`performance_summary` propagated it — so one unaccountable day removed the series, the costs, the
monthly returns, the peak and every other figure from the Performance screen, on 14 of the 29
shipped broker samples. Negative capital was not refused at all: it chained a negative factor, and
one real sample reported −187.9 %, which no long-only portfolio can do.

XIRR had already answered this question for itself: flows that never change sign have no rate, and
the command reports that as an absent figure rather than as a failure.

## Decision

- A sub-period is chained only when both ends mean something: the starting capital is **positive**
  and the ending value is **not negative**. Anything else is skipped, contributing no factor, and
  the chain resumes at the next sub-period that qualifies. TWR is therefore never below −100 %.
- A window in which **every** sub-period was skipped while the portfolio did hold something has no
  return to state: `Error::Math`, because 0 % would read as "it went nowhere" when what happened is
  that no capital this ledger accounts for was ever at work. A window in which the portfolio was
  simply empty throughout keeps its flat 0 %.
- Every caller that draws a screen reports that as an **absent figure**, never as a failure:
  `performance_summary` carries `twr: Option<Decimal>` (and annualises only what it has), the
  assistant's `portfolio_performance` answers `null`, and a plugin's `performance` read does the
  same. Only `Error::Math` degrades; a missing price is still an error.
- The skipping is silent in the figure itself. What is missing is a ledger, and the app already
  says so where it can be fixed: the import's `SaleExceedsHoldings` and the gaps banner (ADR-0089),
  and the deposits the user can add.

## Alternatives

- **Count the appearing value as an external flow.** Arithmetically the same as skipping for that
  one sub-period, but it would also enter `summary`/XIRR as money the user paid in, which it is not.
- **Chain from the first day capital is positive.** The same rule stated only for the leading edge;
  a hole in the middle of the window would still break the figure.
- **Clamp a negative factor at zero (a total loss).** The chain is a product, so one incomplete day
  would hold the whole figure at −100 % for the rest of the window.
- **Report 0 % when nothing could be chained.** Indistinguishable from a portfolio that stood still,
  and the number would be quoted as if it were measured.
- **Refuse to draw the screen, as before.** The screen holds a dozen figures that are correct, and
  the one that is not is the one the user cannot fix without another file.

## Consequences

- A trades-only import now shows the whole Performance screen, with the return itself as `—`.
- A portfolio whose cash went negative for part of a window reports the return of the rest of it;
  the skipped stretch is not visible in the figure, only in the calculation sheet's rows.
- TWR is bounded below by −100 %, so `annualize` no longer has a negative growth factor to refuse.
- `CalculationSheet` keeps a plain `twr` and still fails for a window with nothing chainable: it is
  a panel explaining the chain, and an empty chain has nothing to explain.
