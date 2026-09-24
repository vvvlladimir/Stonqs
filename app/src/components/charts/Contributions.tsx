import type { CSSProperties } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Bar, Rate } from "../ui";

/** One position's share of the portfolio result. */
export interface Contribution {
  key: string;
  label: string;
  /** Contribution as a fraction, e.g. `0.012` is +1.2 percentage points. */
  value: number;
}

/**
 * Who made the result: contributions ranked, on one shared scale, with the total beneath.
 *
 * The cascade this used to draw put every bar at a different offset, which is the one thing a
 * reader does not need here — the question is who contributed how much, and a row is answered
 * by the length of its bar. Losses run to the left of the zero line, so a loss can never be
 * mistaken for a gain of the same size.
 */
export function Contributions({ items, total }: { items: Contribution[]; total?: number }) {
  const { t } = useLingui();
  if (items.length === 0)
    return (
      <p className="muted">
        <Trans>No contributions for this period.</Trans>
      </p>
    );

  const sum = total ?? items.reduce((acc, item) => acc + item.value, 0);
  // Both sides of zero share one span, so the axis is comparable across the whole list.
  const up = Math.max(0, ...items.map((item) => item.value));
  const down = Math.max(0, ...items.map((item) => -item.value));
  const span = up + down || 1;
  const zero = (down / span) * 100;
  // The widest single value, for the reading that starts every bar at the left edge.
  const peak = Math.max(up, down) || 1;

  return (
    <div className="wfall">
      {/* The zero rule belongs to the block, not to each row: drawn per row, the gaps between
          rows cut it into dashes. It is only there when something is actually negative. */}
      <div className={`wfall__rows${zero > 0 ? " wfall__rows--signed" : ""}`}>
        {items.map((item) => {
          const size = (Math.abs(item.value) / span) * 100;
          const positive = item.value >= 0;
          return (
            <div className="wfall__row" key={item.key}>
              <span className="wfall__name" title={item.label}>
                {item.label}
              </span>
              <span className="wfall__lane">
                {/* Two geometries, and the box picks one (`styles/ui/bar.css`): signed against
                    the zero, or linear from the left edge where a lane is too short to hold
                    both sides and halving it again would leave nothing to compare. */}
                <span
                  className="wfall__bar"
                  style={
                    {
                      "--sl": `${positive ? zero : zero - size}%`,
                      "--sw": `${Math.max(size, 0.6)}%`,
                      "--ll": "0%",
                      "--lw": `${Math.max((Math.abs(item.value) / peak) * 100, 0.6)}%`,
                    } as CSSProperties
                  }
                >
                  <Bar size="lg" fill="100%" tone={positive ? "pos" : "neg"} />
                </span>
              </span>
              <Rate value={item.value} digits={2} className="wfall__v" tone />
            </div>
          );
        })}
      </div>

      <div className="wfall__foot">
        <span>{t`Total`}</span>
        <Rate value={sum} digits={2} tone />
      </div>
    </div>
  );
}
