# 28: A result is split into the instrument's and the currency's share

- Status: Accepted

## Context

A euro-based portfolio holding US shares gets one number back: "+240 €". It answers neither of
the two questions actually being asked — did the share rise, or did the dollar? Portfolio
Performance reports four figures side by side: realized and unrealized gain, and realized and
unrealized *currency* gain.

Everything needed was already stored. `Position` keeps `cost_basis` in the settlement currency
next to `cost_basis_base` at the historical rate, and lots never revalue. What was missing was
the subtraction, and a second FX rate: the settlement currency's, which is not necessarily the
quote currency's.

## Decision

The currency's share of a result is what the money put in is worth at the later rate, minus
what it was worth when it was paid:

```
currency_gain = cost_in_settlement_currency × rate(later) − cost_in_base_at_purchase_rate
instrument_gain = total_result − currency_gain
```

- Unrealized: `PositionValuation::currency_gain_base`, using `cost_fx_rate` — the settlement
  currency's rate on the valuation date. `PortfolioValuation` carries the sum.
- Realized: `RealizedGain::currency_gain_base`, fixed at the disposal's own rate, never
  rewritten when rates move later — the same rule as a trade's FX rate.
  `Holdings::realized_currency_gain_base` carries the sum.

`cost_fx_rate` is the quote rate when the two currencies coincide, one when the cost was paid
in the base currency, and a lookup otherwise. A missing rate is `Error::MissingMarketData`,
like every other missing rate — not a silently zero currency gain.

A disposal settled in a different currency than the purchase has no pair to compare and reports
no currency gain; the whole result stays with the instrument. This is rare (a broker switching
settlement currency mid-position) and inventing a rate for it would be worse than declining to
split.

The arithmetic lives in `valuation.rs` and `holdings.rs`, never in the frontend: it is `Decimal`
money, and the UI boundary forbids that.

## Alternatives

- **Splitting by the quote currency.** Wrong for the common European case: an Irish-domiciled
  ETF quoted in EUR holding US assets is not a currency exposure *this* portfolio can see, while
  a USD-settled purchase of it is.
- **Revaluing lots at the current rate.** That destroys the historical cost basis every tax
  report depends on.
- **Computing it in the UI from `fx_rate` and `cost_basis`.** Forbidden by
  `.claude/rules/ui-boundary.md`, and the settlement rate is not on the wire anyway.

## Consequences

- `value_holdings` may need one extra FX lookup per position — only when the settlement currency
  is neither the base nor the quote currency.
- The identity `instrument_gain + currency_gain = total result` holds exactly in `Decimal`, so
  the two columns always add up to the one already shown.
- Base-currency portfolios report a currency gain of exactly zero, which is correct and costs
  no lookup.
