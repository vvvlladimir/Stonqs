# 18: Period presets belong to the core

- Status: Accepted

## Context

Every analysis screen shows a period axis. Until now two kinds of axis existed side by side:
`Performance` and `Risk` asked the host for `period_ranges` and rendered the presets it returned,
while `Income` kept its own list of day windows (30/90/365/1095/"all") and subtracted days from
today in TypeScript, using `1900-01-01` to mean "all history".

That second axis is a small calendar engine in the UI. It disagrees with the core on what a
window is: `PeriodPreset::range` clamps the start to the first transaction (a "5 years" window on a
one-year-old portfolio is one year), it resolves month ends properly, and it is unit-tested.
Two implementations also mean two cache keys for the same question, so the dashboard's income
widget and the income screen could not reuse each other's results.

## Decision

`calc::periods::PeriodPreset` is the only place that turns a preset into dates. It gained
`OneMonth` and `ThreeMonths` so the income screen's windows exist there; `period_ranges` returns
them, shortest first. The frontend keeps labels (`lib/periods.ts`) and one control
(`components/domain/PeriodControl`), never date arithmetic — screens pass a `PeriodRange` straight
to a query hook, and both income queries go through `useIncomeOver`, which keys on the range.

Windows that are not a report period — the 90-day sparkline, which is a quote range — are not
presets and stay where they are used.

## Alternatives

- **Keep the UI's day windows.** Cheaper, but leaves the inception clamp, month-end handling and
  the "1900-01-01 means everything" convention duplicated in TypeScript and untested.
- **Add a `days` parameter to `period_ranges`.** Turns a closed list of presets into an open
  numeric API; the label, the cache key and the comparison window then have no single owner.

## Consequences

- Adding a period is a core change plus one label, not a new list in a screen.
- Every screen with a period offers the same seven presets, so the axis reads the same everywhere;
  the segmented control uses short labels and `.page__controls` scrolls when it does not fit.
- The preset set is wire format: new variants are additive, but renaming one breaks stored UI state
  (`Widget.cfg.period`), so variants are added, never renamed.
