import type { PositionReturnRow } from "../../lib/types";

/** Sort by contribution so the period's main drivers appear first. */
export function byContribution(rows: PositionReturnRow[]): PositionReturnRow[] {
  return [...rows].sort((a, b) => Number(b.contribution) - Number(a.contribution));
}
