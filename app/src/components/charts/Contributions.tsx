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

  return (
    <div className="wfall">
      <div className="wfall__rows">
        {items.map((item) => {
          const size = (Math.abs(item.value) / span) * 100;
          const positive = item.value >= 0;
          return (
            <div className="wfall__row" key={item.key}>
              <span className="wfall__name" title={item.label}>
                {item.label}
              </span>
              <span className="wfall__lane">
                {zero > 0 && <i className="wfall__zero" style={{ left: `${zero}%` }} />}
                <span
                  className="wfall__bar"
                  style={{ left: `${positive ? zero : zero - size}%`, width: `${Math.max(size, 0.6)}%` }}
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
