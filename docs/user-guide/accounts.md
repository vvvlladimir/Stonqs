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
