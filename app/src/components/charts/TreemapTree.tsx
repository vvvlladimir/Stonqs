import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Chart } from "./Chart";
import { Trail } from "./Trail";
import { fullLabel } from "./labels";
import { slotClass, slotFor, squarify, type Rect } from "../../lib/plot";
import { formatMoney, formatPercent } from "../../lib/format";
import type { AllocationBucket } from "../../lib/types";

/** Full-depth treemap: categories provide area and leaves occupy it. */
const HEIGHT = 460;
/** Minimum tile sizes for labels. */
const FITS_NAME = { width: 46, height: 26 };
const FITS_VALUE = { width: 74, height: 58 };
/** Estimated character width used to fit labels. */
const CHAR_EM = 0.58;
const NAME_SIZE = 12.5;
const VALUE_SIZE = 11.5;
const PAD = 8;
const CORNER = 6;

/** Leaf tile and its hierarchy path. */
interface Tile {
  key: string;
  rect: Rect;
  bucket: AllocationBucket;
  /** Path from the root to the leaf. */
  path: AllocationBucket[];
  slot: number;
}

function fit(text: string, width: number, size: number): string {
  const max = Math.floor(Math.max(width, 0) / (size * CHAR_EM));
  if (max <= 1) return "";
  return text.length <= max ? text : `${text.slice(0, max - 1)}…`;
}

/** Recursively lays out children inside each parent rectangle. */
function layout(
  buckets: AllocationBucket[],
  frame: Rect,
  slotOf: (bucket: AllocationBucket, index: number) => number,
  path: AllocationBucket[],
  depth: number,
  out: Tile[],
): void {
  const shown = buckets.filter((bucket) => Number(bucket.weight) > 0);
  if (shown.length === 0) return;
  const rects = squarify(
    shown.map((bucket) => Math.max(Number(bucket.weight), 0)),
    frame,
  );
  shown.forEach((bucket, i) => {
    const rect = rects[i];
    if (!rect || rect.width < 1 || rect.height < 1) return;
    // Descendants keep the root category's palette slot.
    const slot = depth === 0 ? slotOf(bucket, i) : slotOfPath(path, slotOf);
    const here = [...path, bucket];
    if (bucket.children.length === 0) {
      out.push({
        key: here.map((b) => b.key).join("/"),
        rect,
        bucket,
        path: here,
        slot,
      });
    } else {
      layout(bucket.children, rect, slotOf, here, depth + 1, out);
    }
  });
}

/** Returns the branch to enter for a clicked tile. */
function step(tile: Tile): string {
  return tile.path.length > 1 ? tile.path[0].key : tile.bucket.key;
}

/** Uses one root color for every tile in a branch. */
function slotOfPath(
  path: AllocationBucket[],
  slotOf: (bucket: AllocationBucket, index: number) => number,
): number {
  return path.length > 0 ? slotOf(path[0], 0) : 1;
}

export function TreemapTree({
  buckets,
  currency,
  slotOf,
  nameOf,
  onPick,
}: {
  buckets: AllocationBucket[];
  currency: string;
  /** Root category palette slot shared with the other hierarchy view. */
  slotOf?: (key: string) => number | undefined;
  /** Optional full security name for the tooltip. */
  nameOf?: (key: string) => string | undefined;
  /** Enters the root branch containing the clicked tile. */
  onPick?: (key: string) => void;
}) {
  const { t } = useLingui();
  const [hover, setHover] = useState<Tile | null>(null);
  if (buckets.length === 0)
    return (
      <p className="muted">
        <Trans>No shares.</Trans>
      </p>
    );

  const slot = (bucket: AllocationBucket, index: number) => slotOf?.(bucket.key) ?? slotFor(index);

  return (
    <div className="tmap" onMouseLeave={() => setHover(null)}>
      <Chart height={HEIGHT} gutter={0} label={t`Classification tree by area`}>
        {(frame) => {
          const tiles: Tile[] = [];
          layout(buckets, { x: 0, y: 0, width: frame.width, height: HEIGHT }, slot, [], 0, tiles);

          return (
            <>
              {tiles.map((tile) => {
                const { rect } = tile;
                const name = rect.width > FITS_NAME.width && rect.height > FITS_NAME.height;
                const value = rect.width > FITS_VALUE.width && rect.height > FITS_VALUE.height;
                const money = formatMoney(tile.bucket.value_base, currency, {
                  digits: 0,
                });
                const share = formatPercent(tile.bucket.weight, { digits: 1 });
                // The tooltip preserves the full name and hierarchy path.
                const trail = [
                  ...tile.path.slice(0, -1).map((b) => b.label),
                  fullLabel(tile.bucket.label, nameOf?.(tile.bucket.key)),
                ].join(" › ");
                return (
                  <g
                    key={tile.key}
                    className={slotClass(tile.slot)}
                    data-tip={`${trail}: ${share} · ${money}`}
                    onMouseEnter={() => setHover(tile)}
                    onFocus={() => setHover(tile)}
                    onClick={onPick ? () => onPick(step(tile)) : undefined}
                    style={onPick ? { cursor: "pointer" } : undefined}
                  >
                    <rect
                      className="tmap__tile"
                      x={rect.x + 1}
                      y={rect.y + 1}
                      width={Math.max(rect.width - 2, 1)}
                      height={Math.max(rect.height - 2, 1)}
                      rx={CORNER}
                    />
                    {name && (
                      <text className="chart__name" x={rect.x + PAD} y={rect.y + 19}>
                        {fit(tile.bucket.label, rect.width - 2 * PAD, NAME_SIZE)}
                      </text>
                    )}
                    {name && (
                      <text className="chart__value" x={rect.x + PAD} y={rect.y + 35}>
                        {fit(share, rect.width - 2 * PAD, VALUE_SIZE)}
                      </text>
                    )}
                    {value && (
                      <text className="chart__value" x={rect.x + PAD} y={rect.y + 51} opacity={0.75}>
                        {fit(money, rect.width - 2 * PAD, VALUE_SIZE)}
                      </text>
                    )}
                  </g>
                );
              })}
            </>
          );
        }}
      </Chart>

      {/* Keep the trail mounted so hover does not shift the map. */}
      <Trail path={hover?.path ?? []} currency={currency} nameOf={nameOf} />
    </div>
  );
}
