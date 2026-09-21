# 57: Income is classified by its payer, not by what a category is worth

- Status: Accepted

## Context

Other trackers show "earnings by taxonomy" beside its earnings widgets: the same
dividends and interest, divided by a classification tree. We classify *value* already
(`allocation_by_taxonomy`, ADR taxonomy rules) but nothing classified *payments*, so the question
"where does my income come from" could only be answered per instrument.

The two readings are not the same measurement. An allocation divides a valuation at a date, and
every subject in it has a value today. A payment is an event in a window: the instrument that made
it may be sold by now, and a category worth a great deal may have paid nothing at all.

## Decision

`calc::income_by_taxonomy(items, nodes, assignments, excluded)` splits income records through a
tree, and is deliberately **valuation-free**:

- A payment is filed under **whoever paid it**: the security for a dividend, the
  `cash:<account>:<currency>` subject for account interest. A subject split 60/40 splits each of
  its payments by the same weights, exactly as its value is split.
- The unassigned remainder of a payer is `UNCLASSIFIED_KEY`; an **excluded** subject leaves the
  tree *and* its total, never the remainder — the same rule allocation follows.
- A node's money is its own plus its subtree's. Its **payment count is not summed up the tree**: a
  dividend split across two children reached the parent once, and counting it twice would make the
  parent's "payments" contradict the total.
- A node's `weight` is its share of the tree's net income. Interest charged is negative, so a
  weight is not guaranteed to lie in `[0, 1]`; the bar clamps and nothing is renormalised.
- A node no payment reached is left out by the frontend rather than drawn at zero — absence and
  zero mean different things here.

It reaches the UI as `income_taxonomy`, a command of its own rather than a field of
`income_summary`: the tree is picked separately, and switching it must not refetch the calendar,
the payers table and the payments grid.

## Alternatives

- **Weight each payment by the payer's classification *at the payment's date*.** Assignments are
  not historised — a tree records what an instrument *is*, not what it was — so this would be a
  second, silently different classification model for one screen.
- **Derive the split on the frontend from `by_security` plus the assignments.** Money arithmetic in
  TypeScript, and it could not classify account interest, which has no instrument.
- **Add `by_taxonomy` to `income_summary`.** One fetch, but every tree change re-runs every rollup
  of the screen, and a screen with no tree pays for a field it never reads.

## Consequences

- An instrument sold inside the window still reports the income it paid while held; an allocation
  of the same tree would not show it at all.
- Cash classification (`cash_classifications`) gains a second job: it decides where account
  interest is reported, not only where a balance sits.
- The assistant reads the same split through `income_breakdown`, so "which part of my portfolio
  pays me" is answered from the one calculation rather than the model adding instruments up.
