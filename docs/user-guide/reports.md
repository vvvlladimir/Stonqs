# Reports

The tax-and-accounting view of a period: what selling realised, what was paid out, and what it all
cost. Three reports behind one control — gains, dividends, charges — each over the chosen period
and under the account picker. Periods end at the date the app is set to, which is today unless the
date picker was moved back.

Every table here is a CSV away from a tax return, and the export follows the report on screen: it
writes what is being looked at, for the window being looked at, to a file the user picks.

**Gains** are *realised* results only: a disposal that happened inside the period. What a holding
is worth today, and what it would make if sold, is the Positions screen. The figures are proceeds,
cost, fees, taxes, the result and the return on cost, broken down by year and by instrument, with
the individual disposals underneath. Which lots a sale consumed depends on the portfolio's cost
basis method — that is what decides the cost side of every line here. A purchase commission has
no line of its own: it is already inside the cost basis. "Of it, FX" is the part of the result the
exchange rate made rather than the instrument, and is always zero for an instrument priced in the
base currency.

**Dividends** covers what was actually received, with tax withheld and what is accrued. In the
by-instrument table, three columns deliberately ignore the period and read the instrument's whole
payment history instead: the payment schedule inferred from the gaps between payments, the yield
over the last twelve months against cost basis, and the yield on cost over everything ever paid. A
position that has been closed has nothing to divide by, so those read "—" rather than zero.

**Charges** are *standalone* fees and taxes — split by year, by transaction kind and by the account
that paid them. Commission paid inside a purchase is not here: it went into that position's cost
basis, and counting it twice would overstate what the portfolio cost. A question about the total
cost *rate* is answered on the Performance screen, which asks the opposite question and does count
that commission.

A period with no transactions has nothing to report, and the screen says so.
