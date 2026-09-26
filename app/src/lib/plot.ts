import type { CSSProperties } from "react";
/** Converts decimal strings only when projecting values into pixel geometry. */
export function toPlotNumber(value: string): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export interface Scale {
  (value: number): number;
}

export function linearScale(domain: [number, number], range: [number, number]): Scale {
  const [d0, d1] = domain;
  const [r0, r1] = range;
  const span = d1 - d0;
  if (span === 0) return () => (r0 + r1) / 2;
  return (value: number) => r0 + ((value - d0) / span) * (r1 - r0);
}

/** Rounded axis ticks using 1/2/5 × 10^n steps. The step is the *nearest* of them, not the
 *  next smaller one, which would double the number of gridlines whenever the range lands
 *  just under a round step. */
export function niceTicks(min: number, max: number, count = 4): number[] {
  if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) return [min];
  const raw = (max - min) / count;
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const normalized = raw / magnitude;
  const step = (normalized >= 7 ? 10 : normalized >= 3 ? 5 : normalized >= 1.5 ? 2 : 1) * magnitude;
  const ticks: number[] = [];
  // Ticks are multiples of the step, so a float step is rebuilt from an integer index.
  const first = Math.ceil(min / step - 0.001);
  for (let k = first; k * step <= max + step / 2; k++) ticks.push(round(k * step, step));
  return ticks;
}

/** Distance between neighbouring ticks; a single tick stands for its own magnitude. */
export function tickStep(ticks: number[]): number {
  if (ticks.length < 2) return Math.abs(ticks[0] ?? 1) || 1;
  return Math.abs(ticks[1] - ticks[0]);
}

/** Clears the float tail a multiplication leaves behind (0.1 × 3 = 0.30000000000000004). */
function round(value: number, step: number): number {
  const digits = Math.max(0, Math.min(Math.ceil(-Math.log10(step)) + 1, 12));
  return Number(value.toFixed(digits));
}

/** Shared numeric dimensions required by SVG attributes. */
export const CH = {
  /** Label sizes matching the CSS type scale. */
  fs: { micro: 10.5, meta: 11.5, sm: 12.5 },
  /** Series, baseline, crosshair, and marker stroke widths. */
  sw: { series: 1.8, ref: 1.4, hair: 1, mark: 2 },
  /** Bar, block, tile, and marker radii. */
  r: { bar: 2, block: 3, tile: 8, dot: 3.5 },
  /** Top, bottom, left, and axis-label padding. */
  pad: { top: 14, bottom: 20, left: 4, label: 7 },
  /** Right gutter sizes for money, numeric, and percentage labels. */
  gutter: { money: 64, num: 44, pct: 46 },
  /** Sparkline, compact, medium, and large chart heights. */
  h: { spark: 22, sm: 180, md: 240, lg: 320 },
} as const;

/** Builds a polyline path with pixel-rounded coordinates. */
export function path(points: Array<[number, number]>): string {
  return points.map(([x, y], i) => `${i ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
}

export interface Bin {
  from: number;
  to: number;
  count: number;
}

/** Creates width-dependent bins for the daily-return histogram. */
export function histogram(values: number[], bins: number): Bin[] {
  if (values.length === 0 || bins <= 0) return [];
  const min = Math.min(...values);
  const max = Math.max(...values);
  // A constant series has no range, so use a safe fallback span.
  const span = max - min || Math.abs(max) || 1;
  const step = span / bins;
  const out: Bin[] = Array.from({ length: bins }, (_, i) => ({
    from: min + i * step,
    to: min + (i + 1) * step,
    count: 0,
  }));
  for (const value of values) {
    const index = Math.min(Math.floor((value - min) / step), bins - 1);
    out[index].count += 1;
  }
  return out;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Lays out descending non-negative values as near-square treemap rectangles. */
export function squarify(values: number[], frame: Rect): Rect[] {
  const total = values.reduce((sum, v) => sum + v, 0);
  if (total <= 0) return values.map(() => ({ ...frame, width: 0, height: 0 }));

  const out: Rect[] = new Array(values.length);
  const scale = (frame.width * frame.height) / total;
  let { x, y, width, height } = frame;
  let index = 0;

  while (index < values.length) {
    const vertical = width >= height;
    const side = vertical ? height : width;
    let row: number[] = [];
    let rowSum = 0;
    let best = Infinity;

    // Keep adding items while the row's worst aspect ratio improves.
    while (index + row.length < values.length) {
      const next = values[index + row.length] * scale;
      const sum = rowSum + next;
      const thickness = sum / side;
      const worst = Math.max(
        ...[...row, next].map((area) => {
          const length = area / thickness;
          return Math.max(thickness / length, length / thickness);
        }),
      );
      if (row.length > 0 && worst > best) break;
      row = [...row, next];
      rowSum = sum;
      best = worst;
    }

    const thickness = rowSum / side;
    let offset = vertical ? y : x;
    for (const area of row) {
      const length = area / thickness;
      out[index++] = vertical
        ? { x, y: offset, width: thickness, height: length }
        : { x: offset, y, width: length, height: thickness };
      offset += length;
    }
    if (vertical) {
      x += thickness;
      width -= thickness;
    } else {
      y += thickness;
      height -= thickness;
    }
  }
  return out;
}

/** Builds a ring sector from clockwise turn fractions and two radii. */
export function ringArc(
  cx: number,
  cy: number,
  inner: number,
  outer: number,
  from: number,
  to: number,
): string {
  const span = to - from;
  if (span <= 0) return "";
  if (span >= 1) {
    // Reverse the inner winding so SVG's nonzero fill rule preserves the hole.
    return [ring(cx, cy, outer, 1), ring(cx, cy, inner, 0)].join(" ");
  }
  const large = span > 0.5 ? 1 : 0;
  const [x1, y1] = polar(cx, cy, outer, from);
  const [x2, y2] = polar(cx, cy, outer, to);
  const [x3, y3] = polar(cx, cy, inner, to);
  const [x4, y4] = polar(cx, cy, inner, from);
  return [
    `M${x1.toFixed(1)},${y1.toFixed(1)}`,
    `A${outer.toFixed(1)},${outer.toFixed(1)} 0 ${large} 1 ${x2.toFixed(1)},${y2.toFixed(1)}`,
    // eslint-disable-next-line lingui/no-unlocalized-strings -- an SVG path segment, not text
    `L${x3.toFixed(1)},${y3.toFixed(1)}`,
    inner > 0
      ? `A${inner.toFixed(1)},${inner.toFixed(1)} 0 ${large} 0 ${x4.toFixed(1)},${y4.toFixed(1)}`
      : "",
    "Z",
  ]
    .filter(Boolean)
    .join(" ");
}

/** Returns a point on a circle; `turn` starts at twelve o'clock. */
export function polar(cx: number, cy: number, r: number, turn: number): [number, number] {
  const angle = (turn - 0.25) * 2 * Math.PI;
  return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)];
}

/** Draws a closed circle using two half-arcs. */
function ring(cx: number, cy: number, r: number, sweep: 0 | 1): string {
  if (r <= 0) return "";
  const left = (cx - r).toFixed(1);
  const right = (cx + r).toFixed(1);
  const y = cy.toFixed(1);
  const rr = `${r.toFixed(1)},${r.toFixed(1)}`;
  return `M${left},${y} A${rr} 0 1 ${sweep} ${right},${y} A${rr} 0 1 ${sweep} ${left},${y} Z`;
}

/** Palette slots the theme defines; anything past the last one wraps back to the first. */
export const SLOT_COUNT = 8;

/** Palette slot as a CSS variable, so a slot never paints the whole element. */
export function slotVar(slot: number | undefined): CSSProperties {
  return slot === undefined ? {} : ({ "--slot": `var(--slot-${slot})` } as CSSProperties);
}

/** Palette slot for the nth item of a list that carries no slot of its own. */
export function slotFor(index: number): number {
  return (((index % SLOT_COUNT) + SLOT_COUNT) % SLOT_COUNT) + 1;
}

/** The one place a palette slot becomes a chart colour: every categorical shape — treemap tile,
 *  sunburst arc — names its colour through this, so one category looks the same in every view.
 *  `index` is only the fallback for data that carries no slot of its own. */
export function slotClass(slot: number | undefined, index = 0): string {
  const pick = slot && slot > 0 ? slot - 1 : index;
  return `slot-${((pick % SLOT_COUNT) + SLOT_COUNT) % SLOT_COUNT}`;
}

/** The plot's geometry: what the chart measured, and the bounds every mark is drawn inside. */
export interface Frame {
  width: number;
  height: number;
  /** Data plot bounds. */
  left: number;
  right: number;
  top: number;
  bottom: number;
  /** Optional lower lane bounds. */
  laneTop: number;
  laneBottom: number;
}

/** Maps evenly spaced series indices to the plot X axis. */
export function indexScale(count: number, frame: Frame): Scale {
  return linearScale([0, Math.max(count - 1, 1)], [frame.left, frame.right]);
}

/** Builds a rounded Y scale with labels aligned on the right. */
export function valueAxis(values: number[], frame: Frame, includeZero = false) {
  const min = Math.min(...values, includeZero ? 0 : Infinity);
  const max = Math.max(...values, includeZero ? 0 : -Infinity);
  const ticks = niceTicks(min, max);
  const y = linearScale(
    [Math.min(min, ticks[0]), Math.max(max, ticks[ticks.length - 1])],
    [frame.bottom, frame.top],
  );
  return { y, ticks, min, max };
}

/**
 * A 0–1 share as a CSS width. A track cannot be more than full or less than empty, and every
 * caller was clamping that by hand — `Math.min(Math.max(Number(x) ?? 0, 0), 1)`, five times,
 * once with the `Number("")` that reads an empty string as zero rather than as absent.
 */
export function trackWidth(share: string | number | null | undefined): string {
  const value = Number(share);
  if (!Number.isFinite(value)) return "0%";
  return `${Math.min(Math.max(value, 0), 1) * 100}%`;
}
