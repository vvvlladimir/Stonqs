import type { ReactNode } from "react";
import type { I18n } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { Async, Bar, Money, Num, Percent, QueryError, Stat } from "../../../components/ui";
import {
  useDashboard,
  useFire,
  useIncomeOver,
  usePerformance,
  usePlans,
  usePositions,
  useRisk,
  useTrades,
} from "../../../lib/queries";
import { periodLabel, pickRange, usePeriodRanges, type PeriodId } from "../../../lib/periods";
import { formatDay, formatMoney } from "../../../lib/format";
import type {
  DateString,
  FireProjection,
  MoneyString,
  PeriodSummary,
  PortfolioValuation,
  RiskMetrics,
  TradesData,
} from "../../../lib/types";
import {
  RATIO_TERMS,
  fireCfg,
  periodOf,
  ratioTerms,
  sourceOf,
  type MetricCtx,
  type RatioData,
  type WidgetProps,
} from "./model";

/** Valuation fields suitable for a money metric tile. */
type MoneyField = {
  [K in keyof PortfolioValuation]: PortfolioValuation[K] extends MoneyString ? K : never;
}[keyof PortfolioValuation];

/**
 * Shared layout for a metric value and its note. `foot` is what the metric itself has to say
 * under the number; the widget's own setting is what decides whether that is what gets shown.
 */
export function Figure({
  ctx,
  value,
  delta,
  tone,
  foot,
}: {
  ctx: MetricCtx;
  value: ReactNode;
  delta?: ReactNode;
  tone?: string;
  foot?: ReactNode;
}) {
  const note = useFoot(ctx, foot);
  return (
    <>
      <div className="kpiline">
        <span className={`kpiline__v num ${tone ?? ""}`}>{value}</span>
        {delta && <span className="kpiline__d num">{delta}</span>}
      </div>
      {note && <div className="w__foot">{note}</div>}
    </>
  );
}

/** What the line under the value says: the metric's own note unless the widget names another. */
function useFoot(ctx: MetricCtx, own: ReactNode): ReactNode {
  const { i18n } = useLingui();
  const range = useRange(ctx.date, ctx.period);
  switch (ctx.foot) {
    case "none":
      return null;
    case "dates":
      return range ? `${formatDay(range.from)} — ${formatDay(range.to)}` : null;
    case "period":
      return range ? periodLabel(i18n, range) : null;
    default:
      return own;
  }
}

function useRange(date: DateString, period: PeriodId) {
  const ranges = usePeriodRanges(date);
  return pickRange(ranges.data, period);
}

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

/**
 * A money figure from the period summary. Separate from [`FromValuation`]: that one reads a
 * balance on a date, this one reads a change between two.
 */
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
  tone?: string;
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
  const query = useTrades(range, ctx.source);
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return <Async query={query}>{(data) => <Figure ctx={ctx} value={pick(data)} foot={foot?.(data)} />}</Async>;
}

/**
 * What the active plans add up to in an average month. Read off the coming year rather than
 * off an interval, so a quarterly plan and a monthly one land on the same scale (ADR-0033).
 */
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

/**
 * One figure over another — a yield, a cost share, a cash quota. Both sides are figures the
 * core computed; the division happens here because a share of two displayed values is a
 * rendering, not money: nothing is added, rounded or stored.
 */
export function RatioWidget({ widget, date, period }: WidgetProps) {
  const { i18n } = useLingui();
  const ctx = { date, period: periodOf(widget, period), source: sourceOf(widget) };
  const { top, bottom } = ratioTerms(widget.cfg);
  const range = useRange(ctx.date, ctx.period);
  const needsPeriod = RATIO_TERMS[top].periodic || RATIO_TERMS[bottom].periodic;

  const dashboard = useDashboard(ctx.date, ctx.source);
  const performance = usePerformance(needsPeriod ? range : undefined, ctx.source);
  const income = useIncomeOver(needsPeriod ? range : undefined, null, ctx.source);

  if (needsPeriod && !range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  // The tile waits on `dashboard` below; a period query that failed would otherwise leave it
  // waiting for an answer that is not coming.
  const failed = needsPeriod ? (performance.error ?? income.error) : null;
  if (failed) return <QueryError error={failed} />;

  const data: RatioData = {
    valuation: dashboard.data?.valuation,
    summary: performance.data?.summary,
    costs: performance.data?.costs,
    income: income.data?.total,
  };
  const numerator = RATIO_TERMS[top].pick(data);
  const denominator = RATIO_TERMS[bottom].pick(data);
  const currency = dashboard.data?.valuation.base_currency ?? "";

  return (
    <Async query={dashboard}>
      {() => (
        <Figure
          ctx={ctx}
          value={<RatioValue top={numerator} bottom={denominator} multiple={widget.cfg.as === "multiple"} />}
          foot={
            numerator !== undefined && denominator !== undefined
              ? `${formatMoney(numerator, currency, { compact: true })} / ${formatMoney(denominator, currency, { compact: true })}`
              : i18n._(RATIO_TERMS[top].label)
          }
        />
      )}
    </Async>
  );
}

/** A ratio has no answer when the divisor is zero, and that is an absent figure, not a zero. */
function RatioValue({
  top,
  bottom,
  multiple,
}: {
  top?: MoneyString;
  bottom?: MoneyString;
  multiple?: boolean;
}) {
  if (top === undefined || bottom === undefined) return <>…</>;
  const divisor = Number(bottom);
  if (!Number.isFinite(divisor) || divisor === 0) return <>—</>;
  const ratio = Number(top) / divisor;
  if (multiple)
    return (
      <>
        <Stat value={ratio} digits={2} />
        <span className="cur">×</span>
      </>
    );
  return <Percent value={String(ratio)} digits={1} />;
}

/**
 * How far the portfolio is from covering a year of spending, and when this pace would get
 * there. Every figure but today's value is an assumption — the return is not this portfolio's
 * measured return and is never read from it.
 */
export function FireWidget({ widget, date, period }: WidgetProps) {
  const { t, i18n } = useLingui();
  const ctx = { date, period: periodOf(widget, period) };
  const assumptions = fireCfg(widget.cfg);
  const query = useFire(assumptions);

  if (assumptions.annual_spending === "")
    return (
      <p className="muted">
        <Trans>Name the yearly spending this portfolio should cover.</Trans>
      </p>
    );

  return (
    <Async query={query}>
      {(data) => {
        const months = data.months_to_target;
        return (
          <>
            <Figure
              ctx={ctx}
              value={<Percent value={data.progress} digits={0} />}
              foot={
                months === null
                  ? t`not within a hundred years at this pace`
                  : months === 0
                    ? t`already there`
                    : i18n._(
                        msg`${plural(Math.round(months / 12), { one: "# year", other: "# years" })} to go`,
                      )
              }
            />
            <Bar fill={`${Math.min(Math.max(Number(data.progress) * 100, 0), 100)}%`} size="lg" />
            <FireNote data={data} />
          </>
        );
      }}
    </Async>
  );
}

function FireNote({ data }: { data: FireProjection }) {
  const { t } = useLingui();
  const currency = data.base_currency;
  return (
    <div className="w__foot">
      {t`${formatMoney(data.current_base, currency, { compact: true })} of ${formatMoney(data.target_base, currency, { compact: true })}`}
      {data.target_date && ` · ${formatDay(data.target_date)}`}
    </div>
  );
}
