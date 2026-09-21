# 22: Reports are a window, not a lifetime

- Status: Accepted

## Context

`reports_summary` took one `as_of` date and rolled up the whole history behind it: realized
gains, dividends and charges by year, plus a per-security breakdown. The Reports screen was
therefore the only analysis page in the app without a period axis — its `Seg` picked which
report to look at, never over what span.

That is backwards for what the screen is for. A realized-gains report exists to answer "what
do I put on the 2024 return", and the answer is a window. It also made the screen's numbers
incomparable with every other screen, which all resolve a `PeriodPreset` through
`period_ranges` (ADR-0018).

The rollups themselves were written against `&Holdings`, so there was nowhere to filter: the
grouping and the source of the records were one function. `income` had already solved this —
`income_between` returns a `Vec<IncomeRecord>` and every `income_by_*` takes a slice.

## Decision

`capital_gains_*`, `charges_*` and `dividends_*` take a slice of records rather than
`&Holdings`, and `realized_between` / `charges_between` join `income_between` as the filters
that produce one. The window rule — inclusive on both ends — is stated once per record kind
and nowhere else.

`reports_summary(from, to)` builds `Holdings` once at `to` and derives both the requested
window and the equal-length window before it, so every tile can name its own change without
the frontend subtracting two money strings. Alongside the rollups it returns the ledgers —
`disposals`, `payments`, `charge_rows` — each a core record flattened with the symbol, name
and account the UI needs to print it.

CSV export is per table, not per tab: `report_export(section, from, to)` where `section`
names one table (`gains.detail`, `charges.account`, …). Cells are quoted properly, because a
security name is imported from a broker file and may carry the separator.

## Alternatives

- **Keep `&Holdings` and add windowed twins** (`capital_gains_by_year_between`). Doubles the
  surface and leaves two spellings of the inclusive-bounds rule.
- **Filter in the Tauri command.** Puts a date predicate over `Decimal`-bearing records in
  `app/`, which the UI boundary rule exists to prevent, and leaves `sq-cli` without it.
- **One CSV per tab with the tables stacked and blank lines between them.** Nothing reads
  that: a mixed-shape CSV is not a table, and the point of the export is to be opened.

## Consequences

- `yield_on_cost` is the one figure on the screen that is not of the window — it divides a
  position's lifetime dividends by its lifetime cost. Its column and its CSV header say so.
- A per-year rollup of a window has only that window's years, so the screen shows the
  "By year" panel only when the window spans more than one.
- `PortfolioAnalytics::capital_gains` / `::dividends` keep their `as_of` shape by passing
  `holdings.realized` / `holdings.income` whole — a lifetime report is the window that is
  everything.
