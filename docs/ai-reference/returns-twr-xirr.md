# Returns: time-weighted and money-weighted

Two returns answer two different questions, and they disagree on purpose.

**Time-weighted return (TWR)** measures the portfolio, not the investor. It chains the return of
each sub-period between cash movements, so paying money in or taking it out changes nothing. It is
what a fund's factsheet reports, and it is the only fair way to compare a portfolio against an
index: neither of them is being judged on when money arrived.

**Money-weighted return (XIRR)** measures what the investor actually earned. It is the single
annual rate that would make every contribution and withdrawal, on the dates they happened, add up
to today's value. Buying more of something just before it rises lifts XIRR and leaves TWR
untouched.

Which to use:

- "How did my portfolio do?" — TWR.
- "How did *I* do?" — XIRR.
- Comparing against a benchmark — TWR, always.

Both are annualised over windows longer than a year, so a 3-year figure is per-year, not total.
When the two are far apart, the gap is the story: it is entirely about the timing of contributions.

XIRR needs at least one payment in and a value out, and it is undefined for a window with no cash
movements at all. A period that starts and ends inside a single holding has a TWR but no meaningful
XIRR; say that rather than reporting a number.

TWR can be absent too, and for one specific reason: it only measures stretches in which the
portfolio actually had capital at work. A file of trades with no deposits behind them — the usual
first import from a broker, since the money that paid for the trades sits in a bank statement
nobody exported — leaves the cash balance below zero, and there is nothing to have earned a return
on. Those stretches are left out of the chain, so the figure covers the rest of the window; when a
window has no such stretch at all, the return is shown as absent rather than as 0 %, because 0 %
would say the portfolio went nowhere when in fact nothing could be measured. Adding the missing
deposits, or importing the account they came from, is what makes the figure appear.

For the same reason TWR never falls below −100 %: a portfolio that owes more than it holds is a
ledger with something missing, not a loss of more than everything.
