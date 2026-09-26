/** Wording shared by the marks that name a bucket: the trail, the treemap and the sunburst. */

import { formatMonth, formatPercent } from "../../lib/format";
import type { CalendarCell } from "./Calendar";
import type { PeriodReturn } from "../../lib/types";

/** Places the ticker before the optional full security name. */
export function fullLabel(short: string, full?: string): string {
  return full && full !== short ? `${short} — ${full}` : short;
}

/**
 * One month of the return heatmap. The tip names the month the way the income calendar's does —
 * a reader hovering two heatmaps in the same board must not be shown a month name in one and a
 * pair of ISO dates in the other — and the month's own bounds are what the grid already says.
 */
export function returnCell(period: PeriodReturn): CalendarCell {
  const [year, month] = period.from.split("-").map(Number);
  return {
    year,
    month,
    value: Number(period.twr),
    text: formatPercent(period.twr, { digits: 0, signed: true }),
    title: `${formatMonth(year, month)}: ${formatPercent(period.twr, { digits: 2, signed: true })}`,
  };
}
