import { Trans, useLingui } from "@lingui/react/macro";
import { Chart, type ChartHeight } from "./Chart";
import { slotClass, squarify, type Rect } from "../../lib/plot";
import { formatPercent } from "../../lib/format";

/** Renders portfolio weights as area-proportional, squarified tiles. */
export interface TreemapItem {
  key: string;
  label: string;
  /** Fraction of the total; sorting is handled by the caller. */
  weight: string;
  value: string;
  /** Palette slot for category identity, not tile order. */
  slot?: number;
}

const HEIGHT = 260;
/** Estimated average character width used to fit SVG labels. */
const CHAR_EM = 0.58;
const NAME_SIZE = 12.5;
const VALUE_SIZE = 11.5;
/** Horizontal label padding inside a tile. */
const PAD = 10;
/** Tile radius, matching input controls. */
const CORNER = 8;
/** Minimum tile sizes for rendering labels. */
const FITS_NAME = { width: 54, height: 30 };
const FITS_VALUE = { width: 78, height: 62 };

/** Truncate labels rather than letting them overlap neighboring tiles. */
function fit(text: string, width: number, size: number): string {
  const max = Math.floor(Math.max(width, 0) / (size * CHAR_EM));
  if (max <= 1) return "";
  return text.length <= max ? text : `${text.slice(0, max - 1)}…`;
}

export function Treemap({
  items,
  height = HEIGHT,
  onPick,
}: {
  items: TreemapItem[];
  height?: ChartHeight;
  onPick?: (key: string) => void;
}) {
  const { t } = useLingui();
  if (items.length === 0)
    return (
      <p className="muted">
        <Trans>No shares.</Trans>
      </p>
    );

  return (
    <Chart height={height} gutter={0} label={t`Portfolio shares by area`}>
      {(frame) => {
        const weights = items.map((item) => Math.max(Number(item.weight), 0));
        const rects: Rect[] = squarify(weights, { x: 0, y: 0, width: frame.width, height: frame.height });

        return (
          <>
            {items.map((item, i) => {
              const rect = rects[i];
              if (!rect || rect.width < 1 || rect.height < 1) return null;
              const name = rect.width > FITS_NAME.width && rect.height > FITS_NAME.height;
              const value = rect.width > FITS_VALUE.width && rect.height > FITS_VALUE.height;
              return (
                <g
                  key={item.key}
                  className={slotClass(item.slot, i)}
                  data-tip={`${item.label}: ${formatPercent(item.weight, { digits: 1 })} · ${item.value}`}
                  onClick={onPick ? () => onPick(item.key) : undefined}
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
                    <text className="chart__name" x={rect.x + PAD} y={rect.y + 21}>
                      {fit(item.label, rect.width - 2 * PAD, NAME_SIZE)}
                    </text>
                  )}
                  {name && (
                    <text className="chart__value" x={rect.x + PAD} y={rect.y + 37}>
                      {fit(formatPercent(item.weight, { digits: 1 }), rect.width - 2 * PAD, VALUE_SIZE)}
                    </text>
                  )}
                  {value && (
                    <text className="chart__value" x={rect.x + PAD} y={rect.y + 53} opacity={0.75}>
                      {fit(item.value, rect.width - 2 * PAD, VALUE_SIZE)}
                    </text>
                  )}
                </g>
              );
            })}
          </>
        );
      }}
    </Chart>
  );
}
