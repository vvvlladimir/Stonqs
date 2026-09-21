# Risk

What the return cost in volatility and in falls, over the chosen period and under the account
picker. Everything here is computed on trading days, not calendar days.

**The figures at the top.** Annualised volatility (daily standard deviation, scaled by the square
root of 252), the Sharpe ratio against a risk-free rate, the maximum drawdown and the current one,
the longest drawdown, the share of winning days, semi-deviation, the annualised return, and the
best and worst single days. Only the first few of these figures are on screen; the rest are one
click away behind a control reading `more figures`, and `Show less` puts them back. Nothing is
dropped — a figure that is not visible has simply not been unfolded.

**Drawdowns are computed from chained returns, not from account value.** This is the important
detail of the screen: measured on value, a deposit would fill the hole and the portfolio would look
as if it had recovered when it had not.

*Maximum* drawdown is the deepest fall from a peak inside the period. *Current* drawdown is how far
below the last peak the portfolio stands at the end of it — not the depth it has been climbing out
of. *Longest* is the one measured in days, and it is rarely the deepest: a shallow hole that never
fills is the one that costs years. An episode still open is counted to the end of the period.

**Sharpe can be absent.** With zero volatility there is nothing to divide by, and the screen says so
instead of printing a number.

**The rolling-volatility window** is a control of its own: 21, 63, 126 or 252 trading days. It
changes the rolling chart only, not the headline volatility, which is computed over the whole period.

The distribution chart buckets daily returns; the number of buckets follows the width of the column
on screen, because a bin is a property of the display and not of the portfolio.

**Deepest drawdowns** are listed by depth rather than by date: "how much can this lose" is answered
by the worst episode, not by the most recent one.

A period with no transactions has no risk to compute, and the screen says that rather than showing
zeros.
