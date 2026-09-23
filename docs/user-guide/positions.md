# Positions

Every instrument currently held, one row each, largest by value first. Value is read as of the
date the app is set to — today unless the user moved the date picker back; anything that is a
*return* is measured over the period chosen in the controls, because a return needs a window and a
holding does not.

The account picker applies: the rows are the instruments held in the accounts currently in view.

**The period control changes the return columns only.** Quantity, price, value and weight are as of
the date in view, no matter what the period says. A user comparing "value" against "return" across two periods
is comparing one fixed number with one moving one.

**Columns are chosen by the user** and there are many — quantity, price, FX rate, value, weight,
day change, the unrealised result split into instrument and currency parts, dividends received,
yield on value and on cost, payment frequency and last payment received, distance from the
all-time high and the high itself, time-weighted return and its annualised form, money-weighted
return, the period's result, fees, taxes, the position's own volatility, semi-deviation, maximum
drawdown and drawdown length over the period, accounts, ISIN, WKN, the user's note, any instrument
attribute defined by the user, and a 90-day sparkline. A column the user has not enabled is not
missing data — it is simply not on screen.

The Columns button opens the list. Columns already shown sit on top, in the order the table draws
them, and are reordered by dragging the handle beside a name or by pressing the arrow keys while
that handle has focus; the rest are grouped by what they are read off — price, levels, value,
result, dividends, costs, risk, the instrument's own record, attributes — and a search box narrows
the list by name. Reset to default puts both the choice and the order back. Each column's heading
explains itself on hover, which is where two figures that look alike are told apart.

Purchase value, purchase price, unrealised and realised result are always offered under a **named**
cost-basis method, `(FIFO)` or `(moving avg)`, never as one unnamed column: a figure whose meaning
followed a setting could not be compared with the one beside it. The two versions differ only for a
position that was partly sold. The method the portfolio is set to under Settings costs nothing extra
to show; asking for the other one makes the app work every position out a second way, so it is
slower on a large portfolio. A choice made before the columns were named this way still points at
the method the portfolio uses.

Search matches ticker, name or ISIN, and the type filter appears only when the portfolio actually
holds more than one type of instrument.

Each row has a menu: open the instrument card, jump to that instrument's transactions, copy its
ticker or its ISIN. Copying an ISIN is unavailable for an instrument that has none, which is
normal for US stocks.

On a narrow screen the same rows become cards; no column is lost, the shape changes.
