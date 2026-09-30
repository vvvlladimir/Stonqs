# Net worth, assets and liabilities

Two totals exist and they are not interchangeable.

- **Portfolio value** is what is invested: instruments at their market price plus the cash on the
  accounts. Every return, risk figure, allocation weight and rebalance plan is about this total
  and only this total.
- **Net worth** is portfolio value plus everything else owned, minus everything owed. A balance
  sheet, not a performance.

Nothing owned or owed outside the portfolio enters a return, a volatility, an allocation weight or
a rebalance trade. When a question mixes the two — "how did my wealth do this year" — the honest
answer separates them: the portfolio returned *x*, and net worth changed by *y*, part of which is
a revaluation somebody typed.

## What these things are

An asset here has no market to price it: property, a vehicle, a collectible, a private company
share, money lent out, cash held somewhere with no ledger in the app. A liability is a debt: a
mortgage, a loan, a credit card, a drawn credit line, tax assessed and unpaid. Both are stored
positive; which way an amount points is a property of its kind, not a sign, so a mortgage of
250 000 is the figure 250 000, subtracted when the total is formed.

Three things are deliberately *not* modelled this way:

- **An instrument with a published price** belongs to the portfolio. Gold with a quote is an
  instrument; gold in a safe with an opinion attached is an asset.
- **Something operations are recorded against** is an account. Entries and transfers make it an
  account, periodic restatement makes it an asset. Only accounts carry contribution limits,
  because a contribution is money that entered the portfolio and a revaluation is neither money
  nor an account.
- **Margin debt at a broker** is negative cash on an account, part of invested capital.

## Dated valuations, read as steps

Every figure is dated, and the value in force on a day is the latest figure dated on or before it.
There is no interpolation, so the net-worth line is a staircase. A thing is absent before its
first valuation — not the same as worth zero — and absent from its closing day onwards, while its
history stays readable. Figures are converted at the **reading date's** rate, not at the rate of
the day the valuation was written; a missing rate makes the reading fail rather than counting the
thing as nothing.

Each figure reports its age in days and is flagged past half a year: "this may no longer hold",
not "this is wrong". The change beside an amount is measured against the previous valuation, both
at the reading date's rate, so it is the revaluation and not a currency move — a difference in
money, never a rate of return.

## Equity and where a debt is going

A liability may name the asset it is secured on. That link produces one figure — the asset's value
minus the debts secured on it — and nothing else; no payment is deducted anywhere, because the
payment is already a withdrawal or a charge on an account. An asset with nothing owed on it has no
equity figure: it would be the same number twice.

A debt with a rate and a monthly payment carries a forward reading: months until it is clear, the
day that lands on, the interest still to pay, and how much is gone since its first figure. The
arithmetic starts from the balance the owner last wrote and never replaces it. When the payment
does not cover the month's interest there is no end date — the debt grows — and every figure of
that reading is absent rather than zero. The contract's own end is carried separately, so "the
schedule says 2049 but this payment gets there in 2047" is answerable. Leverage is everything owed
over everything owned, investments included; measured, never enforced, with no shipped threshold.

## What it cannot answer

How a flat "performed", what equity is worth after selling costs, or what a debt's balance will
actually be next year — the schedule describes what would happen if nothing changed, and one extra
payment makes it wrong. A stale valuation makes net worth quietly wrong; the age of each figure is
reported, and an answer leaning on an old one should say so.
