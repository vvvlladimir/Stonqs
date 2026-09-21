import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Chart } from "./Chart";
import { Trail, fullLabel } from "./Trail";
import { ringArc, slotClass, slotFor } from "../../lib/plot";
import { useSize } from "../../lib/useSize";
import { formatMoney, formatPercent } from "../../lib/format";
import type { AllocationBucket } from "../../lib/types";

/** Hierarchical allocation view: angle is weight and ring is depth. */
const HEIGHT = 460;
/** Outer padding keeps labels away from the chart edge. */
const PAD = 10;
/** Hub radius reserved for the total and upward navigation. */
const HUB = 0.2;
/** Minimum ring thickness for labels. */
const FITS_RADIAL = 34;
/** Minimum arc length measured at the label's inner edge. */
const FITS_ARC = 10;
/** Estimated character width used to fit radial labels. */
const CHAR_EM = 0.58;
const NAME_SIZE = 12;
/** Label inset from the ring's inner edge. */
const TEXT_PAD = 6;

/** Ring sector and its hierarchy path. */
interface Sector {
  key: string;
  bucket: AllocationBucket;
  /** Path from the visible root to this node. */
  path: AllocationBucket[];
  depth: number;
  /** Clockwise turn fractions from twelve o'clock. */
  from: number;
  to: number;
  slot: number;
  /** Whether the sector extends to the outer edge. */
  leaf: boolean;
}

function fit(text: string, length: number, size: number): string {
  const max = Math.floor(Math.max(length, 0) / (size * CHAR_EM));
  if (max <= 1) return "";
  return text.length <= max ? text : `${text.slice(0, max - 1)}…`;
}

/** Recursively divides each parent angle by child value. */
function layout(
  buckets: AllocationBucket[],
  from: number,
  to: number,
  slotOf: (bucket: AllocationBucket, index: number) => number,
  path: AllocationBucket[],
  depth: number,
  out: Sector[],
): number {
  const shown = buckets.filter((bucket) => Number(bucket.value_base) > 0);
  const total = shown.reduce((sum, bucket) => sum + Number(bucket.value_base), 0);
  if (total <= 0) return depth;

  let deepest = depth;
  let cursor = from;
  shown.forEach((bucket, i) => {
    const span = ((to - from) * Number(bucket.value_base)) / total;
    const here = [...path, bucket];
    // Descendants keep the root category's palette slot.
    const slot = depth === 0 ? slotOf(bucket, i) : slotOf(path[0], 0);
    out.push({
      key: here.map((b) => b.key).join("/"),
      bucket,
      path: here,
      depth,
      from: cursor,
      to: cursor + span,
      slot,
      leaf: bucket.children.length === 0,
    });
    if (bucket.children.length > 0) {
      const reached = layout(bucket.children, cursor, cursor + span, slotOf, here, depth + 1, out);
      deepest = Math.max(deepest, reached);
    }
    cursor += span;
  });
  return deepest;
}

export function Sunburst({
  buckets,
  currency,
  total,
  slotOf,
  nameOf,
  onPick,
  onUp,
}: {
  buckets: AllocationBucket[];
  currency: string;
  /** Preformatted total for the visible hierarchy level. */
  total: string;
  /** Root category palette slot shared with the treemap. */
  slotOf?: (key: string) => number | undefined;
  /** Full security name used in tooltips. */
  nameOf?: (key: string) => string | undefined;
  /** Enters the root branch containing the clicked sector. */
  onPick?: (key: string) => void;
  /** Moves one level up when the hub is clicked. */
  onUp?: () => void;
}) {
  const { t } = useLingui();
  const [hover, setHover] = useState<Sector | null>(null);
  const { ref, width } = useSize<HTMLDivElement>();
  if (buckets.length === 0)
    return (
      <p className="muted">
        <Trans>No shares.</Trans>
      </p>
    );
  const height = width > 0 ? Math.min(HEIGHT, width) : HEIGHT;

  const slot = (bucket: AllocationBucket, index: number) => slotOf?.(bucket.key) ?? slotFor(index);

  return (
    <div className="sun" ref={ref} onMouseLeave={() => setHover(null)}>
      <Chart height={height} gutter={0} label={t`Classification tree as rings`}>
        {(frame) => {
          const sectors: Sector[] = [];
          const depth = layout(buckets, 0, 1, slot, [], 0, sectors);
          const rings = depth + 1;

          const cx = frame.width / 2;
          const cy = frame.height / 2;
          const outer = Math.min(frame.width, frame.height) / 2 - PAD;
          const hub = outer * HUB;
          const step = (outer - hub) / rings;

          return (
            <>
              {sectors.map((sector) => {
                const r0 = hub + sector.depth * step;
                // Extend leaves so shallow branches still reach the outer edge.
                const r1 = sector.leaf ? outer : r0 + step;
                const mid = (sector.from + sector.to) / 2;
                const money = formatMoney(sector.bucket.value_base, currency, { digits: 0 });
                const share = formatPercent(sector.bucket.weight, { digits: 2 });
                const name = fullLabel(sector.bucket.label, nameOf?.(sector.bucket.key));
                const trail = [...sector.path.slice(0, -1).map((b) => b.label), name].join(" › ");
                // Require enough radial and arc space before rendering a label.
                const arc = (sector.to - sector.from) * 2 * Math.PI * (r0 + TEXT_PAD);
                const fits = r1 - r0 > FITS_RADIAL && arc > FITS_ARC;
                // Flip labels on the far half so they remain readable.
                const flipped = mid > 0.5;
                const angle = mid * 360 - 90 + (flipped ? 180 : 0);

                return (
                  <g
                    key={sector.key}
                    className={slotClass(sector.slot)}
                    data-tip={`${trail}: ${share} · ${money}`}
                    onMouseEnter={() => setHover(sector)}
                    onFocus={() => setHover(sector)}
                    onClick={onPick ? () => onPick(stepIn(sector)) : undefined}
                    style={onPick ? { cursor: "pointer" } : undefined}
                  >
                    <path className="sun__arc" d={ringArc(cx, cy, r0, r1, sector.from, sector.to)} />
                    {fits && (
                      <text
                        className="chart__name"
                        transform={`translate(${cx.toFixed(1)},${cy.toFixed(1)}) rotate(${angle.toFixed(2)})`}
                        x={flipped ? -(r0 + TEXT_PAD) : r0 + TEXT_PAD}
                        y={0}
                        dy="0.32em"
                        textAnchor={flipped ? "end" : "start"}
                      >
                        {fit(sector.bucket.label, r1 - r0 - 2 * TEXT_PAD, NAME_SIZE)}
                      </text>
                    )}
                  </g>
                );
              })}

              {/* The hub shows the visible total and navigates upward. */}
              <g className="sun__hub" onClick={onUp} style={onUp ? { cursor: "pointer" } : undefined}>
                <circle cx={cx} cy={cy} r={hub} />
                <text className="chart__value" x={cx} y={cy} dy="0.32em" textAnchor="middle">
                  {fit(formatMoney(total, currency, { compact: true }), hub * 1.8, NAME_SIZE)}
                </text>
              </g>
            </>
          );
        }}
      </Chart>

      <Trail path={hover?.path ?? []} currency={currency} nameOf={nameOf} />
    </div>
  );
}

/** Returns the branch to enter for a clicked sector. */
function stepIn(sector: Sector): string {
  return sector.path.length > 1 ? sector.path[0].key : sector.bucket.key;
}
