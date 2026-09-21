# 19: CSV import detection is per language and arbitrated by values

- Status: Accepted

## Context

Import was verified against one broker (Trade Republic). Running 29 real broker
exports from 12 countries through the parser showed that half the files produced
no date format at all, that a file-level decimal separator silently corrupted
rows written the other way (`-24.999996` read as `-24999996`), and that header
aliases misfired in both directions: a Swedish `Instrumentvaluta` was mapped as
the instrument, and columns named `Belopp`, `Bedrag`, `Importo` or `Time` were
not mapped at all. Almost every file also used its own wording for operations.

The tempting fix is a table per broker — the shape other trackers ship, as one
converter per broker. With users worldwide and no way to collect their files, such a table
only ever covers the brokers we happen to have seen.

## Decision

Detection carries two kinds of knowledge and no third:

1. **Dictionaries per language.** Header aliases and operation wording are
   ordered word lists covering the languages users write in, never a rule keyed
   to a broker's file name or column order.
2. **Shapes of values.** Where a language cannot decide, the column's own values
   do: a header match is ranked exact > whole word > substring, and a weak match
   is vetoed when the values contradict the field. A column no dictionary names
   can still be recognised (an ISIN column called `identifier`).

Per-file settings — encoding, header row, trailing total line, date format,
decimal separator — are detected when the caller leaves them unset, and the
detected value is returned in `ParsedCsv::config` so the UI shows it and can
override it. Detection never demands one shape for a whole file: a broker
changes its export mid-history, so the majority wins and the disagreeing cell is
parsed on its own with a warning.

## Alternatives

- **A converter or column table per broker.** Highest accuracy for a file we
  have, nothing for a file we do not. Rejected as the primary mechanism; the
  same effect is available to users through broker templates, which they create
  from their own file and which are data, not code.
- **Ask the user for every setting.** Honest, and what the wizard falls back to,
  but a 30-column export with 25 operation kinds is not a questionnaire anybody
  finishes.
- **Trust headers only** Simple, but a header can name the wrong thing:
  an `instrument` alias then claims `Instrumentvaluta` too.

## Consequences

- Adding a language is a list entry; adding a broker is not a code change at all.
- A recognised operation kind is a *proposal*: keyword matches land in
  `ImportMapping::kind_aliases`, which the user edits, so nothing is applied
  invisibly.
- Wording we cannot read stays unmapped on purpose. `Corporate action` could be a
  dividend or a split, and guessing would forge the answer.
- The detection is only as good as the corpus it is measured against, so the
  matrix (`sqcli import scan <dir>`) is part of the workflow: a change to a
  dictionary is judged by rows-ready across every sample file, not by one file.
