# Accounts

Where the portfolio's money and instruments actually sit, and the place to define the slices the
account picker offers.

**Two kinds of account, and the difference matters.**

- A *deposit* account holds cash in one currency. Dividends, interest, fees and the cash leg of
  every trade land here.
- A *securities* account (a depot) holds instruments. It names a deposit account as its settlement
  account: that is where the money for its purchases comes from and where its income arrives.

This split is why looking at a depot alone shows price performance only — the dividends settled on
the deposit account beside it. A user surprised that "my account earned nothing" is usually looking
at one half of a pair.

Each card shows the account's value in base currency, its currency, its kind, the settlement
account behind a depot, and how many transactions it carries. A dash instead of a value means a
quote or an exchange rate is missing, which is not the same as zero.

**Closed and outside the portfolio.** An account can be marked inactive, and an account can sit
outside the portfolio — it is still kept and still lists its transactions, but the portfolio's
figures do not include it.

**Deleting an account takes its transactions with it.** The confirmation says how many, because
that is the whole risk of the action.

**Groups** are saved slices: a name over a set of accounts. They are what turns "my pension, both
depots and their cash" into one choice in the account picker instead of four clicks every time.
Deleting a group deletes nothing but the grouping.

**Contribution limits** are a yearly ceiling on what one account may take — an ISA, a 401(k), an
ИИС. You state the account, the amount and the day the limit year opens, because that is not always
1 January: the UK's allowance year begins on 6 April, so a deposit on 5 April belongs to the
previous one. The app ships no country's rules and no ceilings of its own, and an account may carry
two limits if two allowances apply to it.

What counts against a limit is money that entered the portfolio through that account: a deposit,
and a transfer whose other leg is not in the ledger. Moving money between two of your own accounts
counts for nothing, and neither does buying instruments with money already sitting in the account —
it was contributed when it arrived. Withdrawals are netted off inside the same year, and a year
that ends net negative reads as nothing used rather than as extra allowance.

**Nothing is ever blocked.** The limit is reported against what was paid in and that is all: no
transaction is refused, no import is stopped.
