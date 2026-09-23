# Performance

How the portfolio did over a chosen period, and what drove it. The account picker applies, and so
does the period control — every figure on this screen is a figure *over a window*. The window ends
at the date the app is set to, so moving that date back reads the same period a year or a decade
ago; see the as-of date topic.

**Two returns sit side by side and they answer different questions.**

- *TWR*, the portfolio return: what the investments did, with the effect of deposits and
  withdrawals taken out. Compare this against an index.
- *XIRR*, the investor return: what the money actually earned given when it was put in. Someone who
  bought heavily right before a fall has a worse XIRR than TWR, and that difference is the timing,
  not the instruments.

Beside them: what was earned over the period in money, the capital invested, the fee and tax rate,
turnover, and the period's high.

**`Earned over the period` and the change beneath it are not the same figure.** The change is what
a statement shows — end value minus what the portfolio was worth when the window opened — and
money paid in during the window is part of it. `Earned` is that change with every deposit and
withdrawal taken back out, so it is the only one of the two that is a result.

A window opens on what was there *before* it, not on its first evening. So a deposit made on the
window's own first day counts as money paid in, not as an opening balance. Over the whole history
that means the window opens at zero and the change is the full value of the portfolio today — the
first purchase is money the user found, not something they started with.

Only the first few of these figures are on screen; the rest are one click away behind a control
reading `more figures`, and `Show less` puts them back. Nothing is dropped — a figure that is not
visible has simply not been unfolded.

**Value and flows** is the chart, and the note on it is the trap it exists to avoid: a rising line
can be a deposit rather than a gain, so contributions run as a track underneath.

**Benchmark instrument** picks what the period is compared against, and any instrument in the
directory can be it. The comparison is two figures rather than a chart: the benchmark's own return
over the period, and the excess — the portfolio's return minus it. A benchmark with less history
than the period does not fail: the comparison starts where the benchmark's prices start, and the
figure says which window was actually compared, so "excess" is never two different windows
subtracted from each other.

**Return by month** is a calendar of monthly TWR with a total per year. A month is chained from its
days rather than computed as a ratio of the two month-end values — otherwise a deposit on the 15th
would show up as a gain.

**Return by position** lists the instruments held inside the period, ordered by contribution.
Contribution is a *share of the portfolio's return*, not an amount of money: a big position moving
a little can outweigh a small position doubling.

**Real TWR** and **Real XIRR** appear only once the portfolio names a price-index region. They are
the two returns above with inflation taken out, so they answer what the money buys rather than how
much of it there is. The hint under Real TWR names the region and how much its prices rose over the
period.

Two things about them surprise people. They are not the nominal figure minus inflation — the app
divides rather than subtracts, which over a long period or a high-inflation economy is a visibly
different number. And their window can end earlier than the period does: a month's price index is
published weeks after the month ends, so the real figures stop at the last published month while
the nominal ones beside them still cover the whole period. The tooltip on Real TWR says which day
it reached.
