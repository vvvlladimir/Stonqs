import type { ReactNode } from "react";
import { slotFor, slotVar, trackWidth } from "../../lib/plot";
import { formatPercent } from "../../lib/format";
import { Legend, LegendItem } from "./Legend";

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
  /** What the segment's tip says. */
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
  /** The same fill stated as a 0–1 share, clamped by `trackWidth`; `fill` wins if both are given. */
  share?: string | number | null;
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
  share,
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
  const filled = fill ?? (share === undefined || share === null ? undefined : trackWidth(share));

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
              data-tip={segment.title}
            />
          ))
        : filled !== undefined && <i className="bar__fill" style={{ width: filled }} />}
      {over && <i className="bar__over" style={{ left: over.left, width: over.width }} />}
      {gap && <i className="bar__gap" style={{ left: gap.left, width: gap.width }} />}
      {tick !== undefined && <i className="bar__tick" style={{ left: tick }} />}
    </div>
  );
}

/** One slice of a whole: what it is, how much of it there is, and what its tip says. */
export interface ShareSlice {
  key: string;
  /** Legend text — a name, a chip, a link to the instrument. */
  label: ReactNode;
  /** Share of the whole, "0.25" = 25%. */
  share: string | number;
  /** Palette slot; falls back to the slice's place in the list. */
  slot?: number;
  /** Plain text for the tip, when `label` is not one — a link cannot go in an attribute. */
  name?: string;
  /** Replaces the share in the tip: for a composition of money, the amount reads better. */
  tip?: string;
}

/**
 * A stacked track and the legend that names it — one shape for every "what is this made of":
 * a plan's split across instruments, a year's income by kind, a level's shares.
 *
 * The segments, the clamping, the fallback colours and the wording of the tip are built here
 * rather than at each call site, which is where they had drifted into three spellings of one
 * thing. Colour is still a palette slot on the track, never a class on the row.
 */
export function ShareBar({
  slices,
  size = "sm",
  width,
  label,
  legend = true,
}: {
  slices: ShareSlice[];
  size?: BarProps["size"];
  /** Width of the track itself, for bars scaled against a peak rather than against their own. */
  width?: string;
  label?: string;
  /** Off where the names are already beside the bar — a table, a row of its own. */
  legend?: boolean;
}) {
  const tipOf = (slice: ShareSlice, subject: string) =>
    slice.tip ?? `${subject}: ${formatPercent(String(slice.share), { digits: 1 })}`;

  return (
    <>
      <Bar
        size={size}
        width={width}
        label={label}
        segments={slices.map((slice, index) => ({
          key: slice.key,
          slot: slice.slot ?? slotFor(index),
          width: trackWidth(slice.share),
          title: tipOf(slice, slice.name ?? (typeof slice.label === "string" ? slice.label : "")),
        }))}
      />
      {legend && (
        <Legend>
          {slices.map((slice, index) => (
            <LegendItem key={slice.key} slot={slice.slot ?? slotFor(index)}>
              {slice.label}
            </LegendItem>
          ))}
        </Legend>
      )}
    </>
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
