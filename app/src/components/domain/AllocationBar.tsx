import { Trans, useLingui } from "@lingui/react/macro";
import { slotFor, slotVar } from "../../lib/plot";
import { Bar, Money, Percent, ShareBar, Swatch } from "../ui";

/**
 * Level shares: a stacked bar plus per-row tracks.
 *
 * Not a pie/donut — close slices are hard to compare by angle and need labels anyway. Bar gives the
 * whole at a glance, per-row tracks let rows compare directly.
 */
export interface BarItem {
  key: string;
  label: string;
  /** Money as a string, as computed by the core. */
  value: string;
  /** Level share: "0.25" = 25%. */
  weight: string;
  /** Palette slot (1-8); falls back to row order. */
  slot?: number;
}

interface Props {
  items: BarItem[];
  /** Level total; omitted, no total row (shown by the sibling panel instead). */
  total?: string;
  currency: string;
  /** Legend rows to list; the bar keeps every segment either way. */
  limit?: number;
  /**
   * The per-row track under each legend line. Off where the block is read at a glance rather
   * than compared row by row — a dashboard tile: the stacked bar above already carries the
   * shares, and a second track per row halves how many rows the tile can show.
   */
  tracks?: boolean;
  /** Row click: drill into a category, or open a security's card. */
  onPick?: (key: string) => void;
}

export function AllocationBar({ items, total, currency, limit, tracks = true, onPick }: Props) {
  const { t } = useLingui();
  const rows = [...items].sort((a, b) => Number(b.value) - Number(a.value));
  if (rows.length === 0)
    return (
      <p className="muted">
        <Trans>Nothing to show.</Trans>
      </p>
    );

  const peak = Math.max(...rows.map((row) => Number(row.weight)), 0.0001);
  // The bar keeps every segment — the whole is the whole — and only the legend is cut.
  const listed = limit ? rows.slice(0, limit) : rows;

  // Its own container: the legend reshapes by ITS width, so the rule holds in a widget tile,
  // in a panel on a screen and in a dialog alike, and no caller declares anything (ADR-0073).
  return (
    <div className="alloc">
      <ShareBar
        label={t`Shares of this level`}
        legend={false}
        slices={rows.map((row) => ({
          key: row.key,
          label: row.label,
          slot: row.slot,
          share: row.weight,
        }))}
      />

      <div className="bars">
        {listed.map((row, i) => (
          <div
            className="bars__row"
            key={row.key}
            style={slotVar(row.slot ?? slotFor(i))}
            role={onPick ? "button" : undefined}
            tabIndex={onPick ? 0 : undefined}
            onClick={onPick ? () => onPick(row.key) : undefined}
            onKeyDown={
              onPick
                ? (e) => {
                    if (e.key === "Enter" || e.key === " ") onPick(row.key);
                  }
                : undefined
            }
          >
            <Swatch />
            <span className="bars__name">{row.label}</span>
            <Money value={row.value} currency={currency} digits={0} className="bars__val" />
            <Percent value={row.weight} digits={1} className="bars__pct" />
            {/* Tracks are scaled to the largest share so small slices stay visible. */}
            {tracks && <Bar size="sm" share={Number(row.weight) / peak} />}
          </div>
        ))}
      </div>

      {total !== undefined && (
        <div className="bars__total">
          <span className="nm">
            <Trans>Total</Trans>
          </span>
          <span className="spacer" />
          <Money value={total} currency={currency} />
        </div>
      )}
    </div>
  );
}
