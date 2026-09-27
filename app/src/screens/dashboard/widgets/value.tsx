import type { ReactNode } from "react";
import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { Async, Money, Percent } from "../../../components/ui";
import { useDashboard, useIncomeOver, usePerformance, usePositions } from "../../../lib/queries";
import { formatDay } from "../../../lib/format";
import type { MoneyString, PeriodSummary, PortfolioValuation } from "../../../lib/types";
import { type MetricCtx, useRange } from "./model";

import { Figure } from "./figure";

type MoneyField = {
  [K in keyof PortfolioValuation]: PortfolioValuation[K] extends MoneyString ? K : never;
}[keyof PortfolioValuation];

export function FromValuation({
  ctx,
  field,
  signed,
}: {
  ctx: MetricCtx;
  field: MoneyField;
  signed?: boolean;
}) {
  const query = useDashboard(ctx.date, ctx.source);
  return (
    <Async query={query}>
      {(data) => (
        <Figure
          ctx={ctx}
          value={
            <Money value={data.valuation[field]} currency={data.valuation.base_currency} signed={signed} />
          }
        />
      )}
    </Async>
  );
}

export function DayMetric(ctx: MetricCtx) {
  const { t } = useLingui();
  const query = usePositions(ctx.date, ctx.source);
  return (
    <Async query={query}>
      {(data) => {
        // No second quote means there is nothing to compare, not zero.
        const known = data.rows.some((row) => row.day_change_base !== null);
        return (
          <Figure
            ctx={ctx}
            value={known ? <Money value={data.day_change_base} currency={data.base_currency} signed /> : "—"}
            foot={known ? undefined : t`no previous quote`}
          />
        );
      }}
    </Async>
  );
}

/** Money fields of the period summary that make a tile on their own. */
type SummaryField = keyof PeriodSummary;

/** A change between two dates, unlike [`FromValuation`]'s balance on one. */
export function FromSummary({
  ctx,
  field,
  signed,
  foot,
}: {
  ctx: MetricCtx;
  field: SummaryField;
  signed?: boolean;
  foot?: (summary: PeriodSummary) => ReactNode;
}) {
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
          value={<Money value={data.summary[field]} currency={data.base_currency} signed={signed} />}
          foot={foot?.(data.summary) ?? `${formatDay(range.from)} — ${formatDay(range.to)}`}
        />
      )}
    </Async>
  );
}

/** A cost rate and the money behind it; the core divides, the tile only renders. */
export function CostRate({ ctx, field }: { ctx: MetricCtx; field: "fee_rate" | "tax_rate" }) {
  const range = useRange(ctx.date, ctx.period);
  const query = usePerformance(range, ctx.source);
  const amount = field === "fee_rate" ? "fees_base" : "taxes_base";
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return (
    <Async query={query}>
      {(data) => {
        const rate = data[field];
        return (
          <Figure
            ctx={ctx}
            value={rate === null ? "—" : <Percent value={rate} digits={2} />}
            foot={<Money value={data.costs[amount]} currency={data.base_currency} />}
          />
        );
      }}
    </Async>
  );
}

export function FromPerformance({
  ctx,
  field,
}: {
  ctx: MetricCtx;
  field: "twr" | "xirr" | "twr_annualized";
}) {
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
      {(data) => {
        const value = data[field];
        return (
          <Figure
            ctx={ctx}
            value={value === null ? "—" : <Percent value={value} signed />}
            foot={`${formatDay(range.from)} — ${formatDay(range.to)}`}
          />
        );
      }}
    </Async>
  );
}

export function IncomeMetric(ctx: MetricCtx) {
  const range = useRange(ctx.date, ctx.period);
  const query = useIncomeOver(range, null, ctx.source);
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
          value={<Money value={data.total.net_base} currency={data.base_currency} />}
          delta={<Money value={data.change_base} currency={data.base_currency} signed />}
          foot={
            <>
              <Trans>
                {plural(data.total.events, { one: "# payment", other: "# payments" })} · previous period
              </Trans>{" "}
              <Money value={data.previous_total.net_base} currency={data.base_currency} />
            </>
          }
        />
      )}
    </Async>
  );
}

/** A drawdown duration: the core counted the days, the tile only picks the wording. */
