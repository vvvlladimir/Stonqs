import type { ReactNode } from "react";
import { Money, Percent } from "../../ui";

/** Absent and rootless both read as a dash. A helper, not a component, to keep fast refresh. */
export function periodCell(value: string | null | undefined): ReactNode {
  if (!value) return dash();
  return <Percent value={value} signed tone={false} />;
}

/** A figure the cost-basis query has not answered for yet reads as an absent one. */
export function money(value: string | undefined, signed = false): ReactNode {
  if (value === undefined) return dash();
  return <Money value={value} signed={signed} tone={false} />;
}

/** An absent figure, dimmed so a column of them does not read as data. */
export function dash(): ReactNode {
  return <span className="dim">—</span>;
}
