import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { formatMoney, formatPercent } from "../../lib/format";
import { Bar, BarKey, Money, Swatch } from "../ui";
import { slotFor, slotVar } from "../../lib/plot";
import type { RebalanceItem } from "../../lib/types";

/** Shows current and target weights on one shared scale. */
export function DriftBars({
  items,
  currency,
  relative = false,
  slotOf,
  onPick,
}: {
  items: RebalanceItem[];
  currency: string;
  /** Show weights relative to the parent category. */
  relative?: boolean;
  /** Palette slot shared with the hierarchy views. */
  slotOf?: (nodeId: string) => number | undefined;
  /** Enters the selected category. */
  onPick?: (nodeId: string) => void;
}) {
  const { t } = useLingui();
  if (items.length === 0)
    return (
      <p className="muted">
        <Trans>The target has no nodes.</Trans>
      </p>
    );

  const now = (item: RebalanceItem) => (relative ? item.relative_current_weight : item.current_weight);
  const goal = (item: RebalanceItem) => (relative ? item.relative_target_weight : item.target_weight);

  const peak = Math.max(...items.map((item) => Math.max(Number(now(item)), Number(goal(item)))), 0.01);
  const percent = (weight: string) => `${(Math.max(Number(weight), 0) / peak) * 100}%`;
  // The core provides monetary drift; this value is only percentage points.
  const points = (item: RebalanceItem) => (Number(now(item)) - Number(goal(item))) * 100;
  const off = items.filter((item) => Math.abs(points(item)) >= 0.05).length;

  return (
    <div className="drift">
      <div className="drift__legend">
        <BarKey>
          <Trans>current weight</Trans>
        </BarKey>
        <BarKey kind="over">
          <Trans>above target</Trans>
        </BarKey>
        <BarKey kind="gap">
          <Trans>short of target</Trans>
        </BarKey>
        <span className="spacer" />
        <span>
          {off ? (
            <Plural
              value={off}
              one={`# of ${items.length} off target`}
              other={`# of ${items.length} off target`}
            />
          ) : (
            <Trans>all on target</Trans>
          )}
        </span>
      </div>

      {/* Keep the wide legend outside the row grid. */}
      <div className="drift__grid">
        {items.map((item, i) => {
          const current = Number(now(item));
          const target = Number(goal(item));
          const over = current > target;
          const slot = slotOf?.(item.node_id) ?? slotFor(i);
          const style = { ...slotVar(slot), ...(onPick ? { cursor: "pointer" } : {}) };
          return (
            <div
              className="drift__row"
              key={item.node_id}
              role={onPick ? "button" : undefined}
              tabIndex={onPick ? 0 : undefined}
              style={style}
              onClick={onPick ? () => onPick(item.node_id) : undefined}
              onKeyDown={
                onPick
                  ? (e) => {
                      if (e.key === "Enter" || e.key === " ") onPick(item.node_id);
                    }
                  : undefined
              }
            >
              <div className="drift__name">
                <div className="drift__name">
                  <Swatch />
                  <span className="drift__label">{item.label}</span>
                </div>

                {/* Whole-unit values keep the column readable row by row. */}
                <Money value={item.current_base} currency={currency} digits={0} className="drift__v" />
              </div>

              {/* The row-sized cell owns hover state; the track stays centered. */}
              <div className="drift__lane">
                <Bar
                  size="lg"
                  fill={percent(String(Math.min(current, target)))}
                  over={
                    over ? { left: percent(goal(item)), width: percent(String(current - target)) } : undefined
                  }
                  gap={
                    over ? undefined : { left: percent(now(item)), width: percent(String(target - current)) }
                  }
                  tick={percent(goal(item))}
                />
              </div>

              <div className="drift__side">
                <span className="drift__pct num">
                  {formatPercent(now(item), { digits: 1 })}
                  <span className="dim"> / {formatPercent(goal(item), { digits: 0 })}</span>
                </span>
                <span className="drift__act">
                  <b className={`drift__delta num ${over ? "is-over" : "is-under"}`}>
                    {points(item) >= 0 ? "+" : "−"}
                    {t`${Math.abs(points(item)).toFixed(1)} pp`}
                  </b>
                  {Math.abs(points(item)) < 0.05
                    ? t`on target`
                    : over
                      ? t`sell ${formatMoney(item.drift_base.replace("-", ""), currency, { digits: 0 })}`
                      : t`buy ${formatMoney(item.drift_base.replace("-", ""), currency, { digits: 0 })}`}
                </span>
              </div>
            </div>
          );
        })}

        {/* Align the shared scale with the track column, not the labels. */}
        <div className="drift__axis">
          <div className="drift__ticks">
            {Array.from({ length: 5 }, (_, k) => (
              <span key={k} className="num">
                {formatPercent(String((peak * k) / 4), { digits: 0 })}
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
