# Net worth

Everything you own and owe, not only what is invested: a flat, a car, valuables, money held
somewhere the app has no ledger for, and debts such as a mortgage, a loan or a credit card.

**Net worth is a second total, not a new value for your portfolio.** Nothing entered here appears
in returns, risk, allocation or rebalancing, and nothing here has a return of its own — a figure
you typed twice, a year apart, is not a performance. *Invested* is the portfolio; *net worth* is
invested plus what else is owned minus what is owed.

**Net worth ignores the account picker.** These things are not accounts, so no lens narrows them.

## Owned, owed, and the figure in force

Add something with **Add asset** or **Add debt**. A debt is entered as what is *still owed*, as a
positive amount — the subtraction is the screen's job, so a minus sign here is only a typo.

Nothing here is priced by a market. Each row carries dated valuations you write yourself, and the
figure in force on any day is the last one dated on or before it, so the line steps rather than
curves: between two valuations nothing was measured. A thing is absent before its first
valuation — the flat you bought in March is not in February's net worth — and absent again from the
day set in **Closed on**, which is how a sold flat or a repaid loan leaves without deleting its
history. Something added but never valued is listed as waiting for its first figure, not as zero.

**Update value** adds a valuation for a day; the same day answered twice replaces that day's
figure. **Value history** lists every figure ever written for one thing and lets you remove one.

**Import valuations…** reads a CSV of figures — one row per thing per day, joined to what you
already have **by name** — and shows what it would write before writing anything, including which
days it would replace. A name the portfolio does not hold is reported rather than created: a file
cannot say what kind of thing it is or which way its amount points. Because one day holds one
figure, importing the same file twice changes nothing. **Export** writes the file this same import
reads back.

Each row shows the age of its figure and says so once it is over half a year old; a banner counts
how many. An old figure is not wrong — only you can tell whether it still holds — so the app
points at it rather than guessing. Beside the amount sits what the last revaluation changed,
against the figure written before it. Both are converted at today's rate, so a currency that moved
in between does not appear here as a gain.

## Debts

**Interest rate** and **Monthly payment** describe a debt and change no balance: what is owed is
whatever you last valued it at. From them the row reads forward — how much is gone since the first
figure, months left, the day it would be clear, the interest still to pay. A payment that does not
cover the month's interest gets no date, because there is none: the debt grows. The end the
contract names is shown beside the computed one, and the two disagreeing is information.

**Secured by** links a debt to the thing it financed. The link moves no number — your payment is
already a withdrawal or a charge on an account, and deducting it here too would count it twice —
but it gives the equity: under the flat, what is left after the debts secured on it. Several debts
on one thing are summed; something with nothing owed on it shows no equity line.

**Debts** also reports what is owed against everything owned, investments included: a leverage
reading, with no threshold built in.

## Account, asset or instrument

Which one something is depends on how you keep it, not on what it is.

- Operations recorded against it — deposits, interest, transfers? An **account**, and contribution
  limits can apply to it.
- Only restated as one figure now and then? It belongs **here**, and no limit can see it.
- A price somebody publishes? An **instrument**, in the portfolio, where it can have a return.

Margin debt at a broker is the one debt that does not belong here: it is negative cash on the
account and part of the invested capital, and leaving it there is what keeps returns right.
