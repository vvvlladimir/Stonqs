import type { ReactNode } from "react";
import type { I18n } from "@lingui/core";
import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { Async, Money, Num, Percent, type Tone } from "../../../components/ui";
import { usePerformance, usePlans, useRisk, useTrades } from "../../../lib/queries";
import { formatDay } from "../../../lib/format";
import type { RiskMetrics, TradesData } from "../../../lib/types";
import { type MetricCtx, useRange } from "./model";

import { Figure } from "./figure";

export function DrawdownDays({ ctx, pick }: { ctx: MetricCtx; pick: (m: RiskMetrics) => number | null }) {
  const range = useRange(ctx.date, ctx.period);
  const query = useRisk(range, "0", "63", ctx.source);
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return (
    <Async query={query}>
      {(data) => {
        const days = pick(data.metrics);
        return (
          <Figure
            ctx={ctx}
            value={days === null ? "—" : <Num>{plural(days, { one: "# day", other: "# days" })}</Num>}
          />
        );
      }}
    </Async>
  );
}

export function FromRisk({
  ctx,
  pick,
  foot,
  tone,
}: {
  ctx: MetricCtx;
  pick: (metrics: RiskMetrics) => ReactNode;
  foot?: (metrics: RiskMetrics, i18n: I18n) => string | undefined;
  tone?: Tone;
}) {
  const { i18n } = useLingui();
  const range = useRange(ctx.date, ctx.period);
  // Same rate and window as the risk widgets, so the history is walked once.
  const query = useRisk(range, "0", "63", ctx.source);
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return (
    <Async query={query}>
      {(data) => (
        <Figure ctx={ctx} value={pick(data.metrics)} tone={tone} foot={foot?.(data.metrics, i18n)} />
      )}
    </Async>
  );
}

/** The period's highest value and how far under it the period ends. */
export function PeakMetric({ ctx }: { ctx: MetricCtx }) {
  const { t } = useLingui();
  const range = useRange(ctx.date, ctx.period);
  const query = usePerformance(range, ctx.source);
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return (
    <Async query={query}>
      {(data) =>
        data.peak === null ? (
          <Figure ctx={ctx} value="—" />
        ) : (
          <Figure
            ctx={ctx}
            value={<Money value={data.peak.value} currency={data.base_currency} />}
            delta={data.peak.distance ? <Percent value={data.peak.distance} signed /> : undefined}
            foot={
              data.peak.distance
                ? t`below the high of ${formatDay(data.peak.date)}`
                : t`reached on ${formatDay(data.peak.date)}`
            }
          />
        )
      }
    </Async>
  );
}

/** Traded volume against the capital at work; the core divides, the tile renders. */
export function TurnoverMetric({ ctx }: { ctx: MetricCtx }) {
  const range = useRange(ctx.date, ctx.period);
  const query = usePerformance(range, ctx.source);
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return (
    <Async query={query}>
      {(data) => (
        <Figure
          ctx={ctx}
          value={data.turnover_rate === null ? "—" : <Percent value={data.turnover_rate} digits={1} />}
          foot={<Money value={data.volume.volume_base} currency={data.base_currency} />}
        />
      )}
    </Async>
  );
}

/** A figure of the trade ledger: the widget names which one, the core computed all of them. */
export function FromTrades({
  ctx,
  pick,
  foot,
}: {
  ctx: MetricCtx;
  pick: (data: TradesData) => ReactNode;
  foot?: (data: TradesData) => ReactNode;
}) {
  const range = useRange(ctx.date, ctx.period);
  const query = useTrades(range, "POSITION", ctx.source);
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return <Async query={query}>{(data) => <Figure ctx={ctx} value={pick(data)} foot={foot?.(data)} />}</Async>;
}

/** The active plans' average month, over the coming year (ADR-0033). */
export function ContributionMetric({ ctx }: { ctx: MetricCtx }) {
  const plans = usePlans();
  return (
    <Async query={plans}>
      {(data) => {
        const owed = data.rows.reduce((sum, row) => sum + row.due_count, 0);
        const active = data.rows.filter((row) => row.plan.active).length;
        return (
          <Figure
            ctx={ctx}
            value={<Money value={data.monthly_base} currency={data.base_currency} />}
            foot={
              owed > 0
                ? plural(owed, { one: "# contribution due", other: "# contributions due" })
                : plural(active, { one: "# active plan", other: "# active plans" })
            }
          />
        );
      }}
    </Async>
  );
}
