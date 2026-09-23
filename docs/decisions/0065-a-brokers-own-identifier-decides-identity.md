# 65: A broker's own identifier decides identity

- Status: Accepted

## Context

Import identity has been a fingerprint of the row's content since ADR-0005: account, date, kind,
instrument, quantity, amount, currency. It makes re-importing the same file a no-op, which is
what it was for, and it answers one question badly — a broker restating an operation.

Restatements are ordinary. A settlement price is corrected, a commission is re-booked, a trade
is cancelled and re-entered with the same identifier. Under a content fingerprint the corrected
row looks like a new operation: the ledger ends up with both, and every figure derived from it
counts the trade twice. The user's only remedy is to find the older row by hand.

Most broker files already name their rows — "Transaction ID", "Order ID", "Booking ID".

## Decision

`Transaction::external_id` holds the broker's own identifier when the file carried one
(`0027_external_id.sql`, indexed, nullable — an operation typed by hand has none and is
recognised by its content exactly as before).

The preview asks the identifier first and the fingerprint second:

- **Same identifier, same fingerprint** — the row is already in the database: `Duplicate`.
- **Same identifier, different fingerprint** — the broker restated it: `RowStatus::Updated`. The
  draft records which stored row it replaces, and committing writes *that* row rather than a new
  one. It is counted apart, as `ImportResult::updated`, because nothing entered the ledger.
- **No identifier** — the content fingerprint decides, unchanged.

A restatement is not covered by the duplicate switch: "write duplicates anyway" is an answer
about rows that look alike, while a restatement is a correction of a row the user already has.
It carries a warning naming the identifier, so a replacement is never silent.

Which column holds the identifier is detected like any other: `ImportField::ExternalId` shares
its header aliases with `LinkId` — brokers print both under "Reference" and "Transaction ID" —
and the values tell them apart. A pairing key repeats (`ValueShape::Link`), an identifier does
not (`ValueShape::Unique`). Detection therefore had to let a field vetoed by the values yield its
column to the next one instead of blocking it.

## Alternatives

**Keep the fingerprint alone and let the user delete the old row.** What happens today. It is
only workable for someone who knows a restatement happened, which is exactly what the app is
better placed to notice.

**A unique constraint on the identifier.** It would make the database refuse the second write
rather than resolve it, turning a correction into an error the user cannot act on. Two brokers
may also use the same identifier, so uniqueness is not even true in general; the column is
indexed, not constrained.

**Compare by identifier only, and always overwrite.** Silent replacement of stored data. The
preview exists to show what a file is about to do; a restatement is exactly the case where it
must be shown.

## Consequences

- Re-importing a corrected statement now converges on the broker's own version instead of
  doubling rows, and the wizard says how many rows it will replace before writing anything.
- `ImportSummary` and `ImportResult` gained a count of their own; a row status is a fifth case
  the CLI and the frontend both render.
- The identifier crosses the wire on every transaction row, so an export carries it and a later
  import can still recognise it.
- Nothing writes an identifier except an import: a transaction typed into the form or created by
  the assistant has none, by design.
