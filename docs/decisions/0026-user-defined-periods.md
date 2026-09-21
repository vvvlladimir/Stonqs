# 26: User-defined periods beside the shipped presets

- Status: Accepted

## Context

ADR-0018 made `calc::periods::PeriodPreset` the only place that turns a period into dates, and
closed the list at seven presets. That answers "every screen reads the same axis" but not "this
portfolio is six years old and I care about the last six years", or "show me 12 May 2024 to
3 June 2026". The seven are a good default set, not a complete one.

Opening the list raises three questions the ADR-0018 design left implicit: who owns the
arithmetic, where a user's period is stored, and what happens to the shipped seven when a user
wants a shorter strip. The third matters because the axis is a segmented control — a user who
adds four periods of their own wants to take some of the seven out of the way, but a preset they
removed by accident must come back.

## Decision

The *shape* of a period stays in the core, only the numbers come from the user.
`calc::periods::PeriodSpec` is the open half of the axis:

- `Relative { unit: Period, count: u32 }` — a window back from the reporting date, counted in the
  same units that already split a range (day/week/month/quarter/year), so "6 years back" reuses
  `Period::before` and its month-end clamping.
- `Fixed { from, to: Option<NaiveDate> }` — a start written down, and an end that is either
  written down too or left open. An open end means "up to the reporting date", so "since
  12 May 2024" is one period rather than a relative window the user has to re-derive every month.
  A written end reaching past the reporting date stops there as well (no quotes exist beyond it);
  a window entirely before the first transaction is an `Err`, not a backwards range, and one
  starting after the reporting date is an `Err` rather than being clamped down to a single day.

Both go through the same inception clamp a preset gets, so a relative window never invents history
the portfolio does not have.

A user's periods are host state (`AppSettings::periods`), not the opaque `ui` blob, because the
host must hand each spec to the core to resolve before the frontend ever sees dates. Removing a
shipped preset writes its code into `AppSettings::hidden_presets` rather than deleting anything —
the same bargain the shipped import layouts make (`import_presets_hidden.json`), with
`periods_restore` bringing all seven back. `period_delete` refuses to empty the strip.

`period_ranges` returns one flat list: shipped presets first, then the user's. Each entry carries
an `id` (a preset code or the user period's own id) and a `name` — `null` for a preset, whose
wording the frontend owns, and the user's own text otherwise, which is data and is never
translated (ADR-0023). A screen picks an id off that list and hands the dates to a query; it never
learns which of the two kinds it picked. A period the current history cannot answer is dropped
from the list rather than offered broken, and `pickRange` falls back to the first entry, so a
stored id that no longer resolves leaves the screen with a period rather than with nothing.

## Alternatives

- **Let the frontend compute user periods.** Puts the inception clamp and month-end arithmetic
  back into TypeScript, which is exactly what ADR-0018 removed, and gives the two halves of one
  axis two different notions of what a window is.
- **Store user periods in the `ui` blob.** Consistent with themes and dashboards, but the host
  cannot stay ignorant of a value it has to resolve against the core on every `period_ranges`.
- **Delete a shipped preset for good.** Simpler state (one list), but a removal with no way back
  is a trap in a control the user is expected to tidy.
- **Make `PeriodPreset` open — a `days` parameter on `period_ranges`.** Rejected in ADR-0018 and
  still wrong: the preset codes are stored UI state (`Widget.cfg.period`) and need one owner.
  Distinct ids for user periods keep that property.

## Consequences

- The axis id is a string, not an enum: `PeriodId` in the frontend, matched against
  `BUILTIN_PRESETS` only where the shipped wording is needed.
- A widget's period override offers the whole axis, so a user period can be picked per widget.
- `AppSettings` gained two fields; both are `#[serde(default)]`, so an older settings file reads
  as "no periods of my own, none hidden".
- Adding another *shipped* preset is still a core change (ADR-0018 stands); adding a personal one
  is now a dialog.
