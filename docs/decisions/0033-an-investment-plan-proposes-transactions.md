# 33: An investment plan proposes transactions, it does not own them

- Status: Accepted

## Context

A regular savings plan ("500 € a month, 60 % into one ETF and 40 % into another") is the way most
private investors actually buy. Other trackers model it as an *Investment Plan*: a security,
an account pair, an amount, an interval, and a button that generates the transactions up to today.

Two things make a direct copy wrong for us.

The first is our importer. Such users mostly type their transactions in, so a
generated buy is the only record of that purchase. Our CSV import reads a broker export by meaning
and is idempotent through `import::fingerprint`; the same purchase therefore arrives a second time,
from the broker, with a different price and a different fee than the one we invented from a quote.
A fingerprint cannot catch it — the two rows have different sources and different numbers. Silent
double counting of a monthly buy is the single worst thing this feature could do.

The second is that the plan has to answer two different questions. "What do I owe this month, and
what would that buy?" is a proposal about the future. "What did the plan actually buy?" is
bookkeeping about the past. Conflating them is what makes a generated transaction hard to undo.

We also want the plan to be usable as an input to rebalancing, where `cash_to_invest` already exists.

## Decision

An investment plan is a **schedule plus a split**, and a *proposal* mechanism on top of it.

- `model::InvestmentPlan` holds the recurrence (`Schedule`), the amount per occurrence, the account,
  and any number of `PlanLeg`s carrying a weight. Zero legs means a cash contribution plan — money
  into the deposit account and nothing bought. A weight is a share of the amount, divided by the sum
  of the plan's weights the way `allocation_by_taxonomy` divides by the sum of included subjects —
  so `60/40` and `6/4` are one plan and a third written as `0.3333` is not rejected for missing 1.
- `calc::plans` turns a plan into `PlannedTransaction`s: pure, price-free arithmetic on top of
  `PriceLookup`/`RateLookup`, exactly like every other module in `calc`. It never writes.
- Nothing is written without the user pressing a button. The screen lists what is due, shows the
  draft transactions it would write, lets them be edited, and only then commits. A committed
  occurrence is recorded in `plan_executions(plan_id, occurrence_date, transaction_id)`.
- "When was this plan last executed" is **derived** from `plan_executions`, never stored on the plan.
  Deleting the generated transaction therefore un-executes the occurrence by itself, and the plan
  offers it again — there is no second piece of state to keep in step.
- `transactions` gains no column. A plan is a fact about intent; a transaction is a fact about money,
  and it must stay readable by every existing query without knowing plans exist.
- A plan's recurrence is its own type (`model::Interval`), not `calc::periods::Period`. `Period` is a
  *reporting axis* — it splits a range into chunks for a chart. `Interval` is a *calendar
  recurrence* — "every 3 months from the 15th". They overlap by accident, not by meaning: `Interval`
  has no `Day` (nobody saves daily) and expresses "every 2 months" and "semi-annually" by a count
  that `Period` has no room for, while `Period` must stay the closed, wire-pinned list of ADR-0018.

## Alternatives

**Generate transactions automatically, as other trackers do.** Rejected: it double-counts
against the CSV import, and the invented price and fee are wrong in a way the user cannot see until
a reconciliation months later.

**Forecast only, never write anything.** Rejected: it leaves the manual-entry user — the one PP
serves best — with no shortcut at all, and a plan that cannot be reconciled against reality can
never tell you that you skipped a month.

**A `plan_id` column on `transactions`.** Rejected: every query and every import path would have to
carry a column that means nothing to them, and a generated row would stop being an ordinary
transaction. A link table keeps the relation where it belongs and costs one join on one screen.

**Store `last_generated` on the plan.** Rejected: two records of the same fact. Deleting the
transaction would leave the plan believing the month was paid.

**One security per plan.** Rejected: the real contribution is one payment split over a portfolio,
and modelling it as three plans makes "did I contribute in March" three questions. One leg with
weight 1 is the PP case, so nothing is lost.

**Let the plan name a target allocation instead of legs.** Rejected for now: the plan could then not
be shown as "this is what I buy", and its execution would change under the user between the preview
and the commit. Feeding `RebalanceOptions::cash_to_invest` from a plan's amount gives the same
answer on the Rebalance screen, where drift is the subject.

## Consequences

- A leftover is expected and is not an error: `amount` divided by a weight rarely buys a whole
  tradable step, so `PlanExecution` reports `cash_left` and the plan does not pretend to spend it.
- A plan whose instrument has no quote on the occurrence date yields `Error::MissingMarketData` for
  that leg like everything else in `calc` — the preview shows the problem rather than a zero.
- Reconciliation against imported transactions (matching a broker's buy to a due occurrence) is
  possible later without a migration: it writes the same `plan_executions` row.
- The projection of future contributions is arithmetic over `Schedule` alone and needs no market
  data, so a FIRE-style widget can be built on it without a provider.
