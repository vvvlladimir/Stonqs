/** Wording shared by the marks that name a bucket: the trail, the treemap and the sunburst. */

import { formatMonth, formatPercent } from "../../lib/format";
import type { CalendarCell } from "./Calendar";
import type { PeriodReturn } from "../../lib/types";

/** Places the ticker before the optional full security name. */
export function fullLabel(short: string, full?: string): string {
  return full && full !== short ? `${short} — ${full}` : short;
}

/** A heatmap month, tipped like the income calendar's months. */
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
