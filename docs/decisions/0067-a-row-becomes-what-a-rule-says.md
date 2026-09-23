# 67: A row becomes what a rule says

- Status: Accepted

## Context

Until now a file row became exactly one operation, and which one was decided by a dictionary
from a wording to a kind. That covers most of every broker file and none of the rest:

- A reinvested dividend is an income *and* a purchase. Mapped to either one, the other is lost.
- One wording spans both directions, told apart by a column the dictionary cannot see. The
  sign heuristic handles the common case and nothing beyond it.
- A broker prints lines that are not operations, and some of them are only recognisable by a
  condition — an empty amount, a zero quantity — rather than by a name on the skip list.

The dictionary also grows in the wrong direction under pressure: shipped layouts already carry
entries like `FINANCE.LOCKUP.DPOSCOMPOUNDINTEREST.CRYPTOWALLET`, and wordings such as
`Sell 3 @ 139.74 USD` cannot be enumerated at all.

## Decision

`ImportMapping::rules` is a list of rules, read before the kind dictionary. A rule is a **match**
and a list of **operations to produce**:

- Conditions are on the *mapped fields*, never on raw column names, so a rule survives a change
  of layout. The tests are a closed set — `equals`, `contains`, `sign`, `present`, `empty` —
  joined by AND, and the first matching rule wins, like every other dictionary here.
- Each emitted operation names its kind and may set fields: a constant, or `{field}` naming
  another value of the same row. There is no arithmetic, no loop and no variable. A rule that
  cannot be read aloud in one sentence is a rule nobody can review, and the one arithmetic case
  that mattered — an amount net of its charges — is already answered by `amount_basis`.
- An empty `emit` leaves the row out, the way a wording on the skip list does: counted, visible,
  reversible.

The parts of one row **share its number** and carry a `part` index. The alternative — one
`ImportRow` holding several drafts — would have changed the shape of the preview, the statuses,
the summary and the commit for a case most files never hit. As separate rows they go through
identity, problems and the commit unchanged, and the wizard shows `12.1` and `12.2`, which is
also how a user reads it: one line of the file, two operations.

Two values the rule author does not write: a broker's identifier gains `#1`, `#2` per part, so a
restatement (ADR-0065) still finds the half it belongs to; and a rule declaring `link` gives its
parts one link id derived from the row, so `calc` reads them as money staying inside the
portfolio and the preview stays reproducible.

Rules are data: they live in a layout, travel with a preset, and are authored either in the file
or through the wizard's kind picker, which offers the two splits that cover the common cases — a
dividend with its purchase, and the two legs of one move — beside the single kinds. No rule
editor ships: the point is to answer the frequent case in one click, not to grow an IDE.

## Alternatives

**A regular expression in the condition, with captures.** The natural next step, and the one case
the closed test set does not cover — a wording that carries the numbers (`Sell 3 @ 139.74 USD`).
Left out here because it needs a regex dependency in the core and belongs with column-level
extraction rather than with row-level rules; nothing in this decision prevents it.

**A scripting language in the layout.** What one would reach for after the third missing feature.
It makes a downloaded layout executable code, which is exactly what the plugin boundary was drawn
to avoid, and makes a preview no longer reviewable by reading it.

**Keep widening the kind dictionary.** It is what the pressure asks for, and it cannot express
"two operations" at any width.

## Consequences

- `ImportRow` gained `part`; a file row can appear more than once in the preview, and the row key
  in the wizard is `number.part`.
- A rule decides the direction itself, so the sign convention does not flip what the rule chose —
  the author already answered that question.
- `kind_aliases` is untouched and stays the common case: a file with no rules previews exactly as
  it did, which the whole existing import suite checks.
- The features Interactive Brokers' reader special-cases — an order superseding its executions, a
  forex trade becoming two linked legs — are the yardstick for whether rules are strong enough.
  They are not expressed as rules today, and doing so would be the honest test of it.
