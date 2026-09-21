import { formatPercent } from "../../lib/format";
import type { CalendarCell } from "../../components/charts";
import type { PeriodReturn, PositionReturnRow } from "../../lib/types";

/** Sort by contribution so the period's main drivers appear first. */
export function byContribution(rows: PositionReturnRow[]): PositionReturnRow[] {
  return [...rows].sort((a, b) => Number(b.contribution) - Number(a.contribution));
}

export function monthCell(period: PeriodReturn): CalendarCell {
  const [year, month] = period.from.split("-").map(Number);
  return {
    year,
    month,
    value: Number(period.twr),
    text: formatPercent(period.twr, { digits: 0, signed: true }),
    title: `${period.from} — ${period.to}: ${formatPercent(period.twr, { digits: 2, signed: true })}`,
  };
}
