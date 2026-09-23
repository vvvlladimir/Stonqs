# The as-of date (the time lens)

Beside the account lens there is a second one: the date the screens are read at. It sits next to
the data source picker and starts on today. Moving it answers "what did the portfolio look like
then" without changing a single stored figure — nothing is written, nothing is recalculated for
keeps, and returning to `Today` restores the live view.

The date is deliberately forgotten when the app closes. Reopening always starts on today, so a
portfolio can never look broken because a date was left behind weeks ago.

## What follows it

Everything that is read *at a moment* or *over a window ending at one*:

- holdings, prices, values and weights;
- allocation and the rebalancing proposal;
- performance, risk, trades, income and reports — their periods end at the chosen date, so
  "1 year" from 3 March 2023 means March 2022 to March 2023;
- the watchlist and the instrument card.

A value at a past date is read from the prices stored for that day, or the last known price before
it. Prices are never looked up forwards in time: a day with no price is answered by the last one
before it, and a date before the instrument's first stored price has no value at all — which reads
as missing data, not as zero.

## What does not follow it

- **Account balances and the transaction list**: they are the record itself, not a reading of it.
- **Instruments, plans and alerts**: an instrument's details, a savings plan and a price rule are
  not statements about a past day.
- **Anything that writes.** A new transaction, a new alert or an imported file is dated by its own
  form, never by the lens. Viewing the past cannot backdate what is entered.

## Talking to the assistant

The assistant answers for **today** even while the screens are set to a past date; it is told which
date the screens are on, so it can say which of the two a figure comes from. If the two could be
confused, ask it explicitly about the date in question.

## A trap worth naming

A past date and a period are different things. The lens is a *point*: the day everything is read
at. A period is a *window*, still chosen separately on the screens that have one. Moving the lens
back a year and asking for "1 year" gives the year that ended then — not the last twelve months.
