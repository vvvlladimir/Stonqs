# 64: A charge carries the currency it was billed in

- Status: Accepted

## Context

A transaction has had one currency since the beginning: `Transaction::currency` covered the
amount, the commission and the tax alike. Real statements do not work that way. A commission is
charged where the order was executed, withholding is deducted where the issuer sits, and the
trade settles on the account's own currency — three currencies on one line is ordinary, not
exotic.

Import had to drop what it could not express: a broker's fee currency column was read as the
row's currency, or simply ignored. Either way the money was wrong. A commission of 12 USD on a
euro trade was recorded as 12 EUR, which inflates a cost basis by the rate and quietly moves
every figure derived from it — cost basis, realised gain, cost rates.

## Decision

`Transaction` gains `fee_currency` and `tax_currency`, both optional. Absent means "the
transaction's own currency", which is what every row written before this decision is, so nothing
already stored changes value and no data migration runs beyond adding the two columns
(`0026_charge_currencies.sql`).

Three rules follow from it:

- **A total never mixes currencies.** `gross_in_transaction_currency` folds in only the charges
  billed in that currency. A foreign charge comes back as its own cash movement
  (`Transaction::foreign_charge_legs`), so a commission billed in dollars leaves the dollar
  balance and not the euro one.
- **Each charge is converted at its own rate.** `calc::holdings::charge_rate` takes the pair of
  the currency the charge was billed in, on the transaction's date. `fx_rate_to_base` stays what
  it has always been — the rate of the *transaction's* currency — and is never lent to a charge
  that belongs to another pair.
- **The cost basis stays complete.** `calc::holdings::Charges` carries a foreign charge back into
  the trade's own currency through the base currency: both rates are of the same day, so this is
  arithmetic over amounts that were actually paid, not a market cross rate invented at lookup
  time. A commission therefore still joins the lot's cost, whichever currency it was taken in,
  and is still not double-counted in `Holdings::fees_base`.

A charge currency equal to the transaction's is stored as `NULL` — one spelling per row, enforced
when writing and folded back when reading, so an older writer cannot produce a second one.

## Alternatives

**Convert at import time.** Record the commission already converted into the trade's currency and
keep only the converted figure. Cheap, no model change — and it throws away what the broker
actually wrote, which is the one thing an import must not do. A rate chosen at import can never
be corrected afterwards.

**A full `Money { amount, currency }` type everywhere.** Correct in the large, and it touches
every number in the code base for a problem that only two fields have. The cost is not paid back
by anything a user would notice.

**Treat a foreign charge as a separate transaction.** Honest, and it doubles the row count of
every import while breaking the rule that one operation is one row — the commission would lose
the trade it belongs to.

## Consequences

- Cash balances, allocation's cash subjects and the journal all gained a second leg to account
  for: a charge billed elsewhere is visible in that currency's balance rather than folded away.
- The scope rewrite (ADR-0044) clears only the charges its rewritten `amount` absorbed. A foreign
  charge stays on the row, because a lens must not make money disappear.
- Import gained `FEE_CURRENCY` and `TAX_CURRENCY` columns, detected like any other currency
  column. A file that repeats the operation's own currency there adds nothing and is folded away.
- The assistant's write tool still takes one figure per charge, in the operation's own currency:
  the catalogue is deliberately narrower than the form.
- A missing rate for a charge's currency is `MissingMarketData` like every other missing rate —
  the operation cannot be valued until the rate arrives, rather than being counted at zero.
