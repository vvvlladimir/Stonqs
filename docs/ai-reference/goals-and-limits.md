# Goals and contribution limits

Two things the user states themselves, neither of which the app measures on its own or enforces.

## A goal

A goal is an amount by a date: "have 40 000 by January 2027". It names the accounts that count
towards it — naming none means the whole portfolio — and it **ignores the account picker**, so a
goal reads the same however the portfolio is being looked at.

What it answers depends on what it was given:

- With a **target date**, it says what would have to be paid in each month to arrive on time.
- With a **monthly amount**, it says when that pace arrives — or that it does not, which is an
  answer rather than a failure.
- With both, it also says whether the stated pace is at least the required one. Without both, that
  question has no answer at all, and "cannot tell" is never shown as "behind".

The expected return is an **assumption the user typed**, not what the portfolio has actually
returned, and it defaults to nothing at all — a goal is saving unless told otherwise. A goal in
another currency is converted at the reading date's rate before it is compared.

A goal is not the financial-independence figure. That one asks what capital a year of spending
would need, from assumptions, with no amount and no date of its own.

## A contribution limit

A limit is a yearly ceiling on what **one account** may take: an ISA, a 401(k), an ИИС. The user
states the account, the amount, the currency and the day the limit year opens — which is not
always 1 January, since the UK's begins on 6 April. A deposit on 5 April therefore belongs to the
year that opened the previous April.

**The app ships no country's rules and no ceilings of its own.** Allowances differ per country and
per year, and a stale figure stated confidently is worse than none, so every number here is one
the user entered. One account may carry two limits if two allowances apply to it.

**It is measured, never enforced.** Nothing refuses a transaction, warns while one is entered, or
blocks an import. The app reports what was paid in against the ceiling and stops there.

## What counts as a contribution

Money that entered the *portfolio* through that account: a deposit, and a transfer whose other leg
is not in the ledger. Moving money between two of the user's own accounts is not a contribution and
never eats an allowance — otherwise one internal transfer would look like a fresh year's allowance
spent. Buying instruments with money already inside the account changes nothing either: the money
was contributed when it arrived, not when it was invested.

Whether a withdrawal counts is the limit's own setting. By default it does not: money taken out
still counts as paid in, because most allowances (an ordinary ISA, an ИИС) stay spent once used —
so an everyday account with frequent spending still shows its deposits as used. With `Withdrawals
give allowance back` on, withdrawals and outgoing transfers are netted off inside the same limit
year, which is how a "flexible" allowance behaves; a year whose withdrawals then exceed its
deposits reads as nothing used — never as extra allowance earned.

One consequence is worth naming: a transfer whose partner leg was never imported counts as a
contribution, because the ledger has no way to see the other side. That is the same reading every
return figure already takes, so the two agree; linking the two legs is what fixes it.
