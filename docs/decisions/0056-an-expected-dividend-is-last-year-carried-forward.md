# 56: An expected dividend is last year's reported payment carried forward

- Status: Accepted

## Context

One tracker's "upcoming dividends" widget reads declared ex- and pay-dates from
DivvyDiary, which needs an API key and an ISIN. Dividend trackers (Snowball, getquin, Sharesight)
show two states instead: *confirmed* when a payment has been declared, *estimated* when it is
projected from past payments, special dividends excluded.

We have no free source of declared future dividends. We do have, per instrument, every dividend its
quote source reported (per share, ex-date, currency; ADR-0034), and the portfolio's own received
payments.

## Decision

`calc::expected_dividends` projects, per open position of the lens:

- The pattern comes from the **provider's reported dividends**, not the user's payments: it covers
  an instrument bought before its first payment, and it is per share.
- Each regular payment of the instrument's last reported year is carried **one year forward** (and
  two, within a two-year horizon), which keeps the payer's calendar instead of chaining a median gap.
  A projected date within half a gap of a date already taken is the same payment and is skipped.
- Amount: monthly and quarterly payers carry the **latest** amount (equal payments, a raise shows);
  half-yearly and yearly payers repeat **each payment's own** amount (interim and final differ).
- A payment above 3× the year's median is special and is not projected. A payer with no schedule
  (`Irregular`, `Unknown`) or silent for more than two gaps is not projected.
- A reported payment whose cash has not arrived yet is listed as `reported`.
- Pay date = ex-date + the median lag measured on this portfolio's own payments; unknown otherwise.
  Net = gross × net/gross of the last payment received; unknown otherwise.
- Today's shares, today's exchange rate — the reading `plan_projection` gives.
- Shares come from the lens; the lag and withholding from the whole portfolio, because the payment
  settles on a deposit account a depot-only lens cannot see (as in `positions_at`).

## Alternatives

- **DivvyDiary / a declared-dividend source.** Accurate for declared payments, but keyed, per-ISIN
  and one more source to budget. Can be added later as a `reported` input without changing the math.
- **Median gap chained from the last ex-date.** Drifts over a year and loses interim/final amounts.
- **Trailing-twelve-months total spread evenly.** Gives a yearly figure, not a date the cash arrives.

## Consequences

- An instrument with manual prices, or one whose source reports no dividends, is never forecast.
- A position bought or sold before the ex-date is not foreseen; the forecast is re-read on every
  change, so it corrects itself once the transaction exists.
- Nothing is stored: the forecast is recomputed from events and transactions, and a new quote
  refresh (which brings new events) invalidates it with the other reports.
