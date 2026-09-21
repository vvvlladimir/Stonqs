# Risk metrics

The portfolio's figures are computed from its daily value series on trading days, not calendar days —
annualisation assumes roughly 252 trading days a year.

**Volatility** is the annualised standard deviation of daily returns. It says how much the value
moves, in either direction. It is not a forecast and not a loss: a portfolio with 15% volatility is
not expected to fall 15%.

**Sharpe ratio** is return above the risk-free rate, divided by volatility — return per unit of
movement. It is only comparable between portfolios measured over the same window, and it is
meaningless over a window of a few weeks.

**Maximum drawdown** is the deepest peak-to-trough fall within the window, as a percentage. It is
computed from chained returns rather than from the raw value, so money paid in during a fall does
not disguise the fall — a portfolio can be worth more than it was at the peak and still be in a
drawdown.

**Semi-deviation** is the part of the movement a holder minds: only the falling days feed it, but
it is measured against every day in the window, exactly as volatility is. That is what makes the
two comparable — a portfolio whose semi-deviation is close to its volatility falls about as hard as
it moves, and one where it is much lower rises more often than it drops. Measuring it against the
falling days alone instead would make a portfolio that rarely falls look like the riskier of two.

**Per position** the same figures are computed from that one holding's daily returns over the
chosen period — the returns its time-weighted return chains, so a purchase or a sale is not a jump
and a dividend counts as return. A day the instrument was not held has no return, so a position
bought last week has a week of statistics however long the period. A sale ends the series; it is
money out, not a crash. The figures are in base currency, so a foreign instrument's volatility
includes the currency's. They need two trading days held, and are empty before that.

Two honest caveats to state when these come up: they are backward-looking descriptions of one
particular window, and they are unstable over short windows. A one-month Sharpe is noise.
