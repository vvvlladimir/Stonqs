import type { DateString } from "./types";

/** Date shortcuts for the as-of lens — days, never periods (ADR-0018). */

/** Last day of the month before this one. */
export function endOfPreviousMonth(date: DateString): DateString {
  const [year, month] = date.split("-").map(Number);
  return new Date(Date.UTC(year, month - 1, 0)).toISOString().slice(0, 10);
}

/** 31 December of the year before this one. */
export function endOfPreviousYear(date: DateString): DateString {
  const year = Number(date.slice(0, 4));
  return `${year - 1}-12-31`;
}
