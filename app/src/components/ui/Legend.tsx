import type { ReactNode } from "react";
import { Swatch } from "./Swatch";

/** The row of "marker — label" pairs that explains a chart, usually in a panel's tools slot. */
export function Legend({ children }: { children: ReactNode }) {
  return <div className="legend">{children}</div>;
}

/** `slot` for palette colours; `color` for markers the palette does not own. */
export function LegendItem({
  slot,
  color,
  children,
}: {
  slot?: number;
  color?: string;
  children?: ReactNode;
}) {
  return (
    <span>
      {color === undefined ? <Swatch slot={slot} /> : <i style={{ background: color }} />}
      {children}
    </span>
  );
}
