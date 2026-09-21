import type { CSSProperties, ReactNode } from "react";

/**
 * A block that scrolls instead of growing: a long list inside a dialog. The height limit is
 * the CSS knob `--scroll-max`, so a screen states how much it gives, not how it scrolls.
 */
export function Scrolly({
  max,
  x,
  chain,
  className,
  children,
}: {
  max?: number;
  /** Scrolls sideways instead of down — a wide table keeps every column reachable.
   *  Together with `max` it scrolls both ways, and the table's sticky header holds. */
  x?: boolean;
  /** Reaching the end hands the scroll back to the page instead of trapping it. */
  chain?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const classes = ["scrolly"];
  if (x) classes.push("scrolly--x");
  if (x && max !== undefined) classes.push("scrolly--xy");
  if (chain) classes.push("scrolly--chain");
  if (className) classes.push(className);
  const style = max === undefined ? undefined : ({ "--scroll-max": `${max}px` } as CSSProperties);
  return (
    <div className={classes.join(" ")} style={style}>
      {children}
    </div>
  );
}
