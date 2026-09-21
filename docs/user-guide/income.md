# Income

Dividends, interest and other money the portfolio received, over the chosen period and under the
account picker. Income settles on deposit accounts, so a scope narrowed to a depot alone will show
little or nothing — that is the scope, not the portfolio.

The headline figures: what was received in the period and how it compares with the previous window
of the same length, tax withheld, what is accrued, and the portfolio yield — income against what
the portfolio is worth. Only the first few of these figures are on screen; the rest are one click
away behind a control reading `more figures`, and `Show less` puts them back. Nothing is dropped — a
figure that is not visible has simply not been unfolded.

**The income-kind filter** narrows everything on the screen to one kind (dividends, interest, and
so on) rather than only the list.

**The calendar always shows the whole history**, not the selected period: seasonality is the thing
it is for, and one year of it is not a pattern. The period control drives the figures above it.
Picking a month opens that month beside its neighbours and against the same month a year earlier.

**What it is made of** and **who pays** split the period by kind and by instrument.

**Where it comes from** reads the same period through one classification tree. Each payment counts
under the category of whoever paid it: a dividend follows the instrument's classification, and
account interest follows how that account's cash balance is classified. An instrument split between
two categories splits its dividends by the same weights, and anything the tree does not classify is
gathered under `not classified`. If more than one tree exists, `Classification` picks which one.

Two things make this different from the shares on the Allocation screen. A category paying nothing
in the period is left out rather than shown at zero, because value is a state and a payment is an
event — a category can be large and pay nothing at all. And an instrument sold during the period
still appears here, under its category, since it did pay while it was held. A category switched off
in that tree is out of this split and out of its total as well.

**Payments** is the feed: date, instrument, amount, as the broker recorded them. Its summary is
worded carefully — "earned" adds up income lines only, because a fee is what the portfolio cost and
money paid in is not a result; "paid in" counts securities delivered in as well as cash, since both
cross the portfolio boundary.

The payments grid can be read by quarter or another column width; the "by payer" table below it is
income only, because a fee has no instrument that paid it and a disposal belongs to the Trades
screen.

A portfolio with no transactions has no income to compute, and the screen says so rather than
showing zeros.
