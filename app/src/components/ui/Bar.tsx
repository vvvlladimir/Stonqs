import type { ReactNode } from "react";
import { slotVar } from "../../lib/plot";

/**
 * Every horizontal track in the product: a single fill, a stacked split, or a
 * weight against its target. Colour comes from a palette slot, never a class on
 * the bar itself, so a slot cannot paint over the surrounding row.
 */

export interface BarSegment {
  key: string;
  /** CSS width, e.g. "42%". */
  width: string;
  /** Palette slot 1-8; falls back to the shared `--slot` of the bar. */
  slot?: number;
  title?: string;
}

/** A span floating over the track, positioned by CSS offsets. */
export interface BarSpan {
  left: string;
  width: string;
}

export interface BarProps {
  /** Single fill as a CSS width; ignored when `segments` are given. */
  fill?: string;
  segments?: BarSegment[];
  /** Palette slot for the fill and for segments that name none. */
  slot?: number;
  /** Sign colour, where the value itself is the meaning rather than a category it belongs to.
   * Still a colour on the track alone: a class on the row would paint the row. */
  tone?: "pos" | "neg";
  /** Hatched overshoot beyond the target. */
  over?: BarSpan;
  /** Dashed shortfall up to the target. */
  gap?: BarSpan;
  /** Target marker at a CSS offset. */
  tick?: string;
  /** Track height: `sm` for an inline cue, `lg` for a row of its own. */
  size?: "sm" | "md" | "lg";
  /** Width of the track itself, for bars scaled against a peak. */
  width?: string;
  /** Segments carry their own colour, so the empty track is not drawn. */
  open?: boolean;
  label?: string;
  className?: string;
}

export function Bar({
  fill,
  segments,
  slot,
  tone,
  over,
  gap,
  tick,
  size = "md",
  width,
  open,
  label,
  className,
}: BarProps) {
  const classes = ["bar", `bar--${size}`];
  if (tone) classes.push(`bar--${tone}`);
  if (open || segments) classes.push("bar--open");
  if (className) classes.push(className);

  return (
    <div
      className={classes.join(" ")}
      style={{ ...slotVar(slot), ...(width ? { width } : {}) }}
      role={label ? "img" : undefined}
      aria-label={label}
    >
      {segments
        ? segments.map((segment) => (
            <i
              key={segment.key}
              className="bar__seg"
              style={{ ...slotVar(segment.slot), width: segment.width }}
              title={segment.title}
            />
          ))
        : fill !== undefined && <i className="bar__fill" style={{ width: fill }} />}
      {over && <i className="bar__over" style={{ left: over.left, width: over.width }} />}
      {gap && <i className="bar__gap" style={{ left: gap.left, width: gap.width }} />}
      {tick !== undefined && <i className="bar__tick" style={{ left: tick }} />}
    </div>
  );
}

/** Legend entry; `kind` picks a swatch that is not a palette slot. */
export function BarKey({ kind, children }: { kind?: "over" | "gap"; children: ReactNode }) {
  return (
    <span>
      <i className={`bar__k${kind ? ` bar__k--${kind}` : ""}`} /> {children}
    </span>
  );
}
