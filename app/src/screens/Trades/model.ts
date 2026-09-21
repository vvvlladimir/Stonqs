import { plural } from "@lingui/core/macro";

/** A holding period is counted in whole days; a same-day round trip is zero of them, not blank. */
export function days(count: number): string {
  return plural(Math.round(count), { one: "# day", other: "# days" });
}
