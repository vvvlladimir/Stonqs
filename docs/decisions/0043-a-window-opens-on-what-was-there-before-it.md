# 43: A reporting window opens on what was there before it

- Status: Accepted

## Context

`period_summary` read the opening balance off the first day of the value series and, to stay
consistent with that, dropped the first day's external flow — the same rule `ValueSeries::twr_points`
uses.

That rule is right for a return and wrong for money. The series' first day is an *end-of-day* value:
it already contains whatever was deposited and bought that morning. Treating it as an opening balance
therefore relabels the first day's deposits as capital that was already there.

On a period that starts at inception the effect is not subtle. A portfolio that began with a
1,388.70 EUR first purchase and grew to 2,436.40 EUR reported an opening balance of 670.35 EUR, a
change of +1,766.05 EUR instead of +2,436.40 EUR, and "added" that was short by the first
contribution. The number a user checks first — how much did this grow — was wrong by the size of the
opening deposit, and the portfolio had not been worth 670.35 EUR on any day before the window.

The same error exists on every window, not only at inception: a deposit made on the opening day of
"1Y" disappeared from `net_flow_base` in exactly the same way.

## Decision

The opening balance is the value of the portfolio **before** the window, derived from the series
without a second valuation:

```
start_value_base = total_value_base[0] - external_flow_base[0]
net_flow_base    = every flow, the first day's included
```

At inception that is zero, because nothing was there. Mid-life it is the real prior value at the
window's opening prices.

`average_capital_base` keeps measuring from `total_value_base[0]`. Money that arrived on the opening
day worked the whole window, which is exactly the assumption `dietz_capital` already makes about
every later flow — its `date > from` filter says so. So the denominator of every cost rate is
unchanged.

`ValueSeries::twr_points` is **not** touched. A return cannot be earned on capital that arrived the
same day, so the first day's flow stays the return's baseline. The money view and the return view
answer different questions and are allowed to open differently.

## Alternatives

- **Prepend a zero-value day to the series.** The arithmetic works, but the first TWR sub-period
  then divides by zero capital, and every consumer of `dates` gains a day that is not in the range
  it asked for.
- **Value the holdings again at `from - 1`.** A true previous close, at the cost of one more
  valuation pass and a new `MissingMarketData` failure for an instrument whose earliest quote falls
  exactly on the window's first day — a screen that worked would start erroring.
- **Special-case inception only.** Leaves the identical bug on every other window and adds a branch
  that has to be threaded to callers that do not know whether their range is the portfolio's first.

## Consequences

`start_value_base`, `net_flow_base` and `absolute_change_base` change on any window whose first day
carries a flow. `delta_base` ("earned"), `invested_capital_base` and `average_capital_base` do not
move at all — the algebra cancels — so no rate, no chart and no other figure shifts with this.

The opening balance is now priced at the window's first-day close rather than the previous one, so a
mid-life window attributes the opening day's price move to the flows rather than to `delta_base`.
That is the same day TWR already excludes, so the two views agree on where the window opens.
