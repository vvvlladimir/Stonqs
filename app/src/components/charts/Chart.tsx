import { useId, useState, type ReactNode } from "react";
import { useSize } from "../../lib/useSize";
import { CH, indexScale, path, slotVar } from "../../lib/plot";
import type { Frame, Scale } from "../../lib/plot";

export type { Frame };
import { axisFormat, formatAxisDate } from "../../lib/format";

/** Shared responsive frame for legends, axes, readouts, and chart interaction. */

/** Legend key; palette colors are defined by CSS classes. */
export interface LegendItem {
  label: string;
  tone: "accent" | "ref" | "in" | "out" | "pos" | "neg";
  /** Palette slot painting a `ref` key, when several reference lines share one chart. */
  slot?: number;
}

/**
 * How tall a chart is: a fixed number of pixels, or `"fill"` — every plot on the dashboard
 * takes that one, so a tile dragged taller is drawn into rather than padded out.
 */
export type ChartHeight = number | "fill";

interface Props {
  height: ChartHeight;
  /** Right-side space for axis labels. */
  gutter?: number;
  label: string;
  legend?: LegendItem[];
  /** Dates enable the shared time axis and crosshair. */
  dates?: string[];
  /** Readout above the chart for the hovered or last point. */
  readout?: (index: number) => ReactNode;
  /** Optional lane height below the plot. */
  lane?: number;
  children: (frame: Frame, hover: number | null) => ReactNode;
}

/** Height reserved for date labels. */
const AXIS = 18;
/** Gap separating the plot from an optional lane. */
const LANE_GAP = 10;

export function Chart({
  height,
  gutter = CH.gutter.num,
  label,
  legend,
  dates,
  readout,
  lane = 0,
  children,
}: Props) {
  // The plot area is what is measured, not the chart: the legend and the readout above it are
  // text of their own height, and a filling chart gets whatever they leave.
  const { ref, width, height: room } = useSize<HTMLDivElement>();
  const [hover, setHover] = useState<number | null>(null);
  const fill = height === "fill";
  const drawn = fill ? room : height;

  const count = dates?.length ?? 0;
  const axis = count > 0 ? AXIS : 0;
  const bottom = drawn - CH.pad.bottom - axis - (lane > 0 ? lane + LANE_GAP : 0);

  const frame: Frame = {
    width,
    height: drawn,
    left: CH.pad.left,
    right: Math.max(width - gutter, CH.pad.left + 10),
    top: CH.pad.top,
    bottom,
    laneTop: lane > 0 ? bottom + LANE_GAP : bottom,
    laneBottom: lane > 0 ? bottom + LANE_GAP + lane : bottom,
  };

  const x = indexScale(count, frame);

  // Dates are evenly spaced, so pointer lookup is a direct index calculation.
  const pick = (clientX: number, target: SVGSVGElement) => {
    if (count === 0) return;
    const box = target.getBoundingClientRect();
    const ratio = (clientX - box.left - frame.left) / Math.max(frame.right - frame.left, 1);
    const raw = Math.round(ratio * (count - 1));
    setHover(Math.min(Math.max(raw, 0), count - 1));
  };

  const marked = hover ?? (count > 0 ? count - 1 : 0);

  return (
    <div className={`chart${fill ? " chart--fill" : ""}`} data-hover={hover === null ? "0" : "1"}>
      {legend && (
        <div className="chart__legend">
          {legend.map((item) => (
            <span key={item.label}>
              <i className={`chart__key chart__key--${item.tone}`} style={slotVar(item.slot)} />
              {item.label}
            </span>
          ))}
        </div>
      )}
      {readout && count > 0 && <div className="chart__readout">{readout(marked)}</div>}

      <div className="chart__plot" ref={ref}>
        {/* A box too short to hold an axis draws nothing: the value scale would run backwards and
            the labels are painted outside the plot, so the chart would land over the page. */}
        {width > 0 && frame.bottom > frame.top && (
          <svg
            width={width}
            height={drawn}
            role="img"
            aria-label={label}
            onPointerMove={(e) => pick(e.clientX, e.currentTarget)}
            onPointerLeave={() => setHover(null)}
          >
            {children(frame, hover)}

            {/* Keep date labels consistent across all time-series charts. */}
            {dates && <DateAxis dates={dates} frame={frame} x={x} />}

            {/* Draw the crosshair last so it spans both plot and lane. */}
            {hover !== null && (
              <line
                className="chart__crosshair"
                x1={x(hover)}
                x2={x(hover)}
                y1={frame.top - 6}
                y2={frame.laneBottom}
              />
            )}
          </svg>
        )}
      </div>
    </div>
  );
}

/** Places two to five date labels according to the available width. */
function DateAxis({ dates, frame, x }: { dates: string[]; frame: Frame; x: Scale }) {
  const slots = Math.min(Math.max(Math.floor((frame.right - frame.left) / 130), 2), 5);
  const y = frame.height - 4;
  const last = dates.length - 1;

  return (
    <>
      {Array.from({ length: slots + 1 }, (_, k) => {
        const index = Math.round((k / slots) * last);
        const anchor = k === 0 ? "start" : k === slots ? "end" : "middle";
        return (
          <text key={index} className="chart__axis" x={x(index)} y={y} textAnchor={anchor}>
            {formatAxisDate(dates[index])}
          </text>
        );
      })}
    </>
  );
}

export function Grid({
  frame,
  ticks,
  y,
  format,
}: {
  frame: Frame;
  ticks: number[];
  y: Scale;
  /** Defaults to the shared numeric style derived from the ticks themselves. */
  format?: (value: number) => string;
}) {
  const label = format ?? axisFormat(ticks);
  return (
    <>
      {ticks.map((tick) => (
        <g key={tick}>
          <line className="chart__grid" x1={frame.left} x2={frame.right} y1={y(tick)} y2={y(tick)} />
          <text className="chart__axis" x={frame.right + CH.pad.label} y={y(tick) + 3.5}>
            {label(tick)}
          </text>
        </g>
      ))}
    </>
  );
}

/** Draws a gradient area with a unique SVG definition per chart instance. */
export function Area({
  points,
  baseline,
  tone = "accent",
}: {
  points: Array<[number, number]>;
  /** Y coordinate used to close the area. */
  baseline: number;
  tone?: "accent" | "neg";
}) {
  const id = useId();
  if (points.length < 2) return null;
  const first = points[0];
  const last = points[points.length - 1];
  // eslint-disable-next-line lingui/no-unlocalized-strings -- an SVG path, not text
  const d = `${path(points)} L${last[0].toFixed(1)},${baseline.toFixed(1)} L${first[0].toFixed(1)},${baseline.toFixed(1)} Z`;

  return (
    <>
      <defs>
        <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" className={`chart__stop chart__stop--${tone}`} stopOpacity={0.28} />
          <stop offset="1" className={`chart__stop chart__stop--${tone}`} stopOpacity={0} />
        </linearGradient>
      </defs>
      <path d={d} fill={`url(#${id})`} />
    </>
  );
}

/** Marks the point currently under the pointer. */
export function Marker({
  x,
  y,
  tone = "accent",
  slot,
}: {
  x: number;
  y: number;
  tone?: "accent" | "ref";
  slot?: number;
}) {
  return (
    <circle
      className={`chart__marker chart__marker--${tone}`}
      style={slotVar(slot)}
      cx={x}
      cy={y}
      r={CH.r.dot}
    />
  );
}
