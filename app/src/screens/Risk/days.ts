import { plural } from "@lingui/core/macro";

/** Compute calendar-day duration from ISO dates without timezone shifts. */
export function dayDelta(from: string, to: string): number {
  const [fy, fm, fd] = from.split("-").map(Number);
  const [ty, tm, td] = to.split("-").map(Number);
  return (Date.UTC(ty, tm - 1, td) - Date.UTC(fy, fm - 1, fd)) / 86_400_000;
}

export function days(from: string, to: string): string {
  return dayCount(dayDelta(from, to));
}

/** A duration the core already counted; only the wording is the interface's business. */
export function dayCount(value: number): string {
  return plural(value, { one: "# day", other: "# days" });
}
