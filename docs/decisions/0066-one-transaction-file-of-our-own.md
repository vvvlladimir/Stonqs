# 66: One transaction file of our own

- Status: Accepted

## Context

Every way operations enter this app ends in the same place — `TransactionDraft`, then the
preview, then the commit — but each way in invents its own shape to get there. A CSV is read by
detection, a Flex statement flattens itself into a table with columns `ibflex` made up, and a
future PDF or API reader would have to make up a third. There is nothing a reader can be asked
to *produce*, so every new reader is a negotiation with the import layer rather than a file.

Neither is there a way out. A portfolio can be read from a broker but not written anywhere, so
moving between profiles, taking a readable backup, or handing the data to a script means going
back to the broker's own exports.

The two problems have one answer, and the competitors show both halves of it. Ghostfolio ships a
canonical activity format and a separate community project of converters into it — and its format
is too poor to carry a commission, which is a live complaint. Portfolio Performance has a hundred
readers and no format at all.

## Decision

`import::canonical` defines **one file the app writes and reads**: `format`, `version`, and rows
carrying the model's own vocabulary — date, operation, account, instrument, quantity, price,
amount, commission and tax with their own currencies, rate, link, the broker's identifier, note.

It is a **reader, not a second import**. `parse_canonical` flattens the rows into the same
`ParsedCsv` every other reader produces and carries a fixed mapping, exactly as `ibflex` does
(ADR-0061), so the wizard, the overrides, duplicate and restatement detection (ADR-0065) and the
commit are the ones every file goes through. A canonical file simply has nothing left to ask.

Three properties are deliberate:

- **Two spellings, one meaning.** JSON is the interchange. A CSV whose headers are these same
  names needs no reader at all: the column names *are* the canonical header aliases, so ordinary
  detection maps them one to one — a test pins that, and a user can produce the format in a
  spreadsheet.
- **No internal ids.** An account travels as its name and an instrument as its ticker and ISIN. A
  file carrying database ids would only ever import back into the database it came from; because
  an account is named, the preview also resolves a name that already exists in the portfolio,
  which is not a guess.
- **Operations only.** Not taxonomies, plans, alerts or settings. A format that grew to cover the
  whole profile would compete with the database file and never stabilise.

Export is the same definition read backwards (`transactions_export`), taking the filter the
screen is showing, because an export is an export of what is on screen.

## Alternatives

**Keep letting each reader invent its columns.** Works, and it is what we do; the cost lands on
the next reader and on anyone wanting their data out.

**Adopt OFX as the canonical shape.** A real standard for exactly this domain, twenty years old,
still exported by American brokers. It is worth *reading* later; as our own write format it is a
poor fit — SGML/XML dialects, vocabulary that does not line up with this model's operations, and
nothing in it for a charge billed in a second currency.

**A full profile backup format.** Larger, and it answers a different question: the encrypted
database file is already the backup, and a backup is not an interchange.

## Consequences

- A new reader — a PDF or API plugin — has something to produce: bytes in, canonical rows out, no
  knowledge of the wizard.
- Round-tripping is testable, and is tested: a portfolio exported and imported into an empty one
  holds the same operations, with charge currencies and broker identifiers intact.
- `ibflex` keeps its own columns for now. Moving it onto the canonical ones is tidying, not a
  correctness matter, and it is not worth churning a reader that works.
- The format is versioned and the reader refuses a newer version rather than reading half of it.
