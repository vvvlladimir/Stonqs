# 21: Contribution divides by the capital at work

- Status: Accepted

## Context

`PositionReturnRow::contribution` answers "how many percentage points of the portfolio's
result came from this security". It divided the position's P/L by the portfolio's value on
the first day of the period.

That denominator only holds for a period without flows. The `SINCE_INCEPTION` period starts at the first
transaction, and a portfolio is usually opened with a token amount — one euro here — so every
position reported thousands of percent. Narrower scopes hid it by accident: a depot-only
scope opens on the first delivery, which is already a real amount.

## Decision

Contribution is `pnl_base / average_capital`, where `average_capital` is the Modified Dietz
denominator: the opening value plus each external flow inside the period weighted by the
share of the period it stayed invested, `(to - flow.date) / (to - from)`.

Without flows the two denominators are identical, so the existing property holds: the
contributions of a flow-free period still sum to the portfolio's return. With flows they sum
to the portfolio's Modified Dietz return — money-weighted, unlike the TWR column beside it.
A non-positive capital yields a zero contribution rather than a sign-flipped ratio.

## Alternatives

- **Mean of the daily portfolio values.** Also a fair "capital at work", but it needs a full
  daily valuation pass, and with no flows it does not equal the opening value, so the
  contributions of a flow-free period would stop summing to that period's return.
- **Weights times returns (Brinson).** Sums to TWR exactly, but needs a daily weight series
  per position and answers a different question than the P/L column it sits next to.

## Consequences

- Contribution is money-weighted, like XIRR and unlike the TWR in the neighbouring column;
  the sum over a period with flows is the portfolio's Modified Dietz return.
- The same position shows a larger contribution in a depot-only scope than in the whole
  portfolio, because cash is capital at work too.
