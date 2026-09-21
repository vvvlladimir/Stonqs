# 61: A Flex statement is a second reader, not a second import

- Status: Accepted

## Context

Interactive Brokers does not publish a usable transaction CSV. What it publishes is a *Flex
Query*: a report definition kept in the broker's web office, exported as XML with every value in
an attribute. It is the format every portfolio tracker built around this broker reads, and it is
structured — typed sections, stable identifiers, an explicit currency and FX rate per row — so
none of the detection the CSV import exists to do applies to it.

The import already owns a long pipeline that has nothing to do with delimiters: the preview, the
per-cell overrides, the kind and symbol and account mappings, the plausibility checks, the
fingerprint that makes a re-import a no-op, and the single-transaction commit. Adding a second
import screen would duplicate all of it and would have to be kept in step with it forever.

Four things in a Flex statement do not map onto the model on their own:

- Asking the query for executions *and* orders prints one purchase twice — once per fill and once
  as the order.
- A forex trade (`assetCategory="CASH"`) is a currency exchange, not a purchase. Read as one row
  it is money crossing the portfolio boundary, which breaks the return.
- An options or futures trade carries a contract multiplier the model has no field for.
- A corporate action's quantity delta would double-count against quotes that are already
  split-adjusted and against the lots a corporate action adjusts.

## Decision

`import::ibflex` is a **reader**, not an import. It flattens the statement's sections into one
table with its own fixed column names and returns the same `ParsedCsv` a delimited file parses
to; `import::parse_file` picks the reader off the bytes, so the file decides and no caller asks.
Everything downstream — `build_preview`, the wizard, the commit — is unchanged.

The layout for those columns is a function in that module rather than a row in
`presets/brokers.json`: the columns are this module's own invention, so there is nothing for a
user to lay out differently. `ImportService::preview` uses it when no mapping was given.

The four traps are answered in the reader:

- The coarser trade level wins whenever the file carries it: with any `ORDER` row present, the
  `EXECUTION` rows are dropped; closed lots and summaries always are.
- A forex trade becomes two rows sharing a link id — `TransferIn` on each side, the negative leg
  flipped to `TransferOut` by the file's own sign — which is the shape `calc` reads as internal.
  The commission goes to the leg whose currency it was charged in.
- A contract with a multiplier other than 1, a cancelled trade and a corporate action are given
  wordings this reader invents and **ships on the skip list**. They stay counted and visible in
  the preview and can be mapped by hand; they are never imported as something they are not.

Dates are normalised to ISO in the reader, because the query's date format is a choice in the web
office that the file itself does not report. `amount_sign` is pinned to `Signed` rather than
voted on: Interactive Brokers signs every amount, and a three-row file has no majority to read.

The automated download (the Flex Web Service: a token, a query id, a reference code and a poll)
is deliberately not part of this. It needs a broker credential in the vault and a background job,
and it is worth nothing until the file import it would feed is correct.

## Alternatives

- **A second import screen for XML.** Duplicates the preview, the overrides, the deduplication and
  the commit, and every later change to any of them has to be made twice.
- **Ask the user for two single-section CSV Flex Queries and ship two presets.** No code at all,
  but the broker must be configured twice and imported twice, and the corporate actions and the
  per-row FX rate are lost either way. Acceptable as a workaround, not as the answer.
- **Read a Flex statement into the model directly, past `build_preview`.** Faster to write and
  gives up the wizard: no preview of what is about to be written, no per-cell correction, no
  account mapping, and a separate deduplication rule to keep in step with the CSV one.
- **Import a corporate action as a delivery.** Doubles a split, because quotes are stored already
  split-adjusted and lots are adjusted by a corporate action.
- **Use the broker's `tradeID` as the deduplication key.** Stronger than the fingerprint for this
  one format, but a stored transaction does not carry it, so the identity would have to be a
  second column on the table and a second rule in the commit. The existing fingerprint already
  makes overlapping exports a no-op.

## Consequences

- Interactive Brokers imports with no parse settings and no column mapping. The only question the
  wizard asks is which account the statement's alias is.
- A new dependency, `quick-xml`, in `core`.
- Options and futures are visibly not imported rather than imported wrongly. Whoever holds them
  sees a counted, named row on the skip list.
- `ParsedCsv` is now the shape *any* broker file parses to, not only a delimited one. A third
  format is another reader and a branch in `parse_file`.
