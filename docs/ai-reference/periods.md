# Periods

A window is always named, never computed. The shipped names are the usual axis — this month, this
quarter, year to date, one year, three years, five years, since inception — and the user may add
their own, either a fixed pair of dates or a rolling window.

Two things follow:

- **Never do date arithmetic.** Ask for a period by its id; the app resolves it against the
  portfolio's own history and the current date. "Last year" as two dates the assistant subtracted
  is a different window from the one the screen shows, and the difference is invisible.
- A period resolves against what exists. "Since inception" starts at the first transaction, not at
  an arbitrary date, and a period reaching back further than the portfolio simply starts where the
  data does.

Annualised figures (XIRR, volatility, Sharpe) are per year regardless of the window's length. Over
a window shorter than a year they are an extrapolation and should be described as one.
