# 24: A cost rate counts the commission the cost basis swallowed

- Status: Accepted

## Context

`Holdings::charges` records standalone `Fee`/`Tax` operations only. That is deliberate and
load-bearing: a buy commission is already inside the position's cost basis, and counting it
again in `fees_base` would report the same euro twice. `.claude/rules/money-and-fx.md` states
the invariant, and the "Charges" report is built on it.

A portfolio fee rate and tax rate ask a different question:
what did owning and trading this portfolio cost over the period, against the capital that was
at work. Answering it from `Holdings::charges` would understate the number by exactly the part
a broker hides where it is least visible — the commission inside a purchase, and the tax
withheld from a dividend before it ever reaches the account.

So the two answers are both right, and neither may be silently substituted for the other.

## Decision

A second function, `calc::costs_paid` (and `costs_paid_by_security`), walks the transactions of
the window directly and sums every `fees`/`taxes` field plus the amount of a standalone
`Fee`/`Tax` operation, with refunds subtracting. It is a separate function rather than a flag
on the existing rollups, so a caller has to name which question it is asking.

`Holdings::charges` and `charges_by_*` are unchanged. The fee and tax rates divide
`costs_paid` by `PeriodSummary::average_capital_base` through `PeriodSummary::rate_of`, which
returns `None` rather than a ratio when no capital was committed.

## Alternatives

- **Add trade commissions to `Holdings::fees_base`.** Breaks the cost-basis invariant: the
  commission would then be inside `cost_basis_base` and in `fees_base`, and every unrealized
  P/L that subtracts one from the other would be wrong.
- **A `include_trade_costs: bool` on the existing rollups.** The default would decide which
  number the "Charges" screen shows, and a reader of the call site could not tell which of the
  two definitions is in play without following the flag.
- **Record trade costs as a third `ChargeRecord` list on `Holdings`.** Cheap to read, but it
  puts a number that must never be summed with `charges` right next to it.

## Consequences

- The "Charges" report and the fee-rate tile disagree on purpose; both tooltips say so.
- `costs_paid` re-resolves FX per transaction instead of reusing the holdings pass. That is one
  extra walk of the window's transactions, not of its days.
- A position's `fees_base`/`taxes_base` columns on the positions table are attributable costs:
  an account-level fee carries no `security_id` and drops out, exactly as in
  `charges_by_security`.
