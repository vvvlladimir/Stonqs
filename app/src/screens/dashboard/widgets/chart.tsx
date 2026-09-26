import { bucketLabel, slotOfNode } from "../../../lib/taxonomy";
import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { AllocationBar } from "../../../components/domain/AllocationBar";
import { Async } from "../../../components/ui";
import { DriftBars } from "../../../components/charts";
import {
  BenchChart,
  Calendar,
  Treemap,
  Underwater,
  ValueChart,
  Contributions,
  returnCell,
  type CalendarCell,
} from "../../../components/charts";
import {
  useAllocation,
  useBenchmarks,
  useInflationSeries,
  useInflationStatus,
  useIncomeOver,
  useIncomeTaxonomy,
  usePerformance,
  usePortfolio,
  usePositionReturns,
  useRebalance,
  useSecurities,
  useTaxonomies,
} from "../../../lib/queries";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { formatMonth, formatMoney, formatPercent, formatRegion } from "../../../lib/format";
import { slotFor } from "../../../lib/plot";
import {
  benchmarksOf,
  inflationOf,
  periodOf,
  taxonomyOf,
  useWidgetRisk,
  useWidgetTarget,
  type WidgetProps,
  sourceOf,
} from "./model";

export function ChartWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const performance = usePerformance(range, source);

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <Async query={performance}>
      {(data) => <ValueChart series={data.series} currency={data.base_currency} height="fill" />}
    </Async>
  );
}

export function BenchWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const ids = benchmarksOf(widget.cfg);

  const performance = usePerformance(range, source);
  const securities = useSecurities();
  const benchmarks = useBenchmarks(ids, range, source);
  const wantsInflation = inflationOf(widget.cfg);
  const inflation = useInflationSeries(wantsInflation ? range : undefined, source);
  const region = useInflationStatus();
  // Slot 1 is the portfolio's own colour, so the reference lines start at the next one.
  const lines = ids.map((id, i) => ({
    label: securities.data?.find((s) => s.id === id)?.symbol ?? "",
    series: benchmarks[i]?.data,
    slot: slotFor(i + 1),
  }));
  const missing = lines.filter((_, i) => benchmarks[i]?.isError).map((line) => line.label);
  // The cost of money is one more date-aligned line: it stops at the last published month
  // rather than running flat to the right edge.
  if (wantsInflation && inflation.data) {
    lines.push({
      label: region.data?.region ? formatRegion(region.data.region) : "",
      series: inflation.data,
      slot: slotFor(ids.length + 1),
    });
  }

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <>
      {/* The tile is useless without a benchmark and silent about it otherwise: the chart it
          draws is just the portfolio's own growth. */}
      {ids.length === 0 && (
        <p className="muted">
          <Trans>No benchmark chosen — pick an instrument in the widget settings.</Trans>
        </p>
      )}
      {missing.length > 0 && (
        <p className="muted">
          <Trans>No prices in this period: {missing.join(", ")}.</Trans>
        </p>
      )}
      <Async query={performance}>
        {(data) => <BenchChart portfolio={data.growth} benchmarks={lines} height="fill" />}
      </Async>
    </>
  );
}

export function DrawdownWidget({ widget, date, period }: WidgetProps) {
  const report = useWidgetRisk(widget, date, period);
  if (!report)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  return <Async query={report}>{(data) => <Underwater series={data.drawdown} height="fill" />}</Async>;
}

export function ContributionWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const rows = usePositionReturns(range, source);

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  // Keep the tail visible so the waterfall still reconciles to the total.
  const count = Number(widget.cfg.count) || 8;

  return (
    <Async query={rows}>
      {(data) => (
        <Contributions
          items={[...data]
            .sort((a, b) => Number(b.contribution) - Number(a.contribution))
            .slice(0, count)
            .map((row) => ({ key: row.security_id, label: row.symbol, value: Number(row.contribution) }))}
        />
      )}
    </Async>
  );
}

export function TreemapWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const { i18n } = useLingui();
  const taxonomyId = taxonomyOf(widget.cfg);
  const allocation = useAllocation(taxonomyId ? "taxonomy" : "security", taxonomyId, date, source);
  const portfolio = usePortfolio();

  const currency = portfolio.data?.base_currency ?? "";

  return (
    <Async query={allocation}>
      {(data) => (
        <Treemap
          items={[...data.buckets]
            .sort((a, b) => Number(b.weight) - Number(a.weight))
            .map((bucket) => ({
              key: bucket.key,
              label: bucketLabel(i18n, bucket),
              weight: bucket.weight,
              value: formatMoney(bucket.value_base, currency, { compact: true }),
            }))}
          height="fill"
        />
      )}
    </Async>
  );
}

export function IncomeCalendarWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const { t } = useLingui();
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const income = useIncomeOver(range, null, source);

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <Async
      query={income}
      isEmpty={(data) => data.by_month.length === 0}
      empty={
        <p className="muted">
          <Trans>No payments in this period.</Trans>
        </p>
      }
    >
      {(data) => {
        const currency = data.base_currency;
        const cells: CalendarCell[] = data.by_month.map((month) => ({
          year: month.year,
          month: month.month,
          value: Number(month.net_base),
          text: formatMoney(month.net_base, currency, { compact: true, symbol: false }),
          title: t`${formatMonth(month.year, month.month)}: ${formatMoney(month.net_base, currency)} · ${plural(month.events, { one: "# payment", other: "# payments" })}`,
        }));
        return (
          <Calendar
            height="fill"
            cells={cells}
            totals={data.by_year.map((year) => ({
              year: year.year,
              text: formatMoney(year.net_base, currency, { compact: true, symbol: false }),
            }))}
          />
        );
      }}
    </Async>
  );
}

/** The monthly-return heatmap: the same grid the performance screen shows, on the dashboard.
 * A monthly TWR is chained inside the month, so a deposit on the 15th is not a gain. */
export function ReturnsWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const performance = usePerformance(range, source);

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <Async
      query={performance}
      isEmpty={(data) => data.monthly_returns.length === 0}
      empty={
        <p className="muted">
          <Trans>The period is shorter than a month.</Trans>
        </p>
      }
    >
      {(data) => (
        <Calendar
          height="fill"
          cells={data.monthly_returns.map(returnCell)}
          totals={data.annual_returns.map((year) => ({
            year: Number(year.from.slice(0, 4)),
            text: formatPercent(year.twr, { digits: 0, signed: true }),
          }))}
        />
      )}
    </Async>
  );
}

export function AllocationWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const { i18n } = useLingui();
  const taxonomyId = taxonomyOf(widget.cfg);
  const allocation = useAllocation(taxonomyId ? "taxonomy" : "security", taxonomyId, date, source);
  const portfolio = usePortfolio();

  return (
    <Async query={allocation}>
      {(data) => (
        <AllocationBar
          items={data.buckets.map((bucket) => ({
            key: bucket.key,
            label: bucketLabel(i18n, bucket),
            value: bucket.value_base,
            weight: bucket.weight,
          }))}
          currency={portfolio.data?.base_currency ?? ""}
          limit={Number(widget.cfg.count) || 8}
          tracks={false}
        />
      )}
    </Async>
  );
}

/** The period's income under the tree's top level. Not the composition widget over a
 * different number: value is a state and a payment is an event, so a category holding
 * nothing may still have paid, and one paying nothing is left out rather than shown at zero. */
export function IncomeTaxonomyWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const { i18n } = useLingui();
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const taxonomies = useTaxonomies();
  const chosen = taxonomyOf(widget.cfg) ?? taxonomies.data?.[0]?.id ?? null;
  const income = useIncomeTaxonomy(chosen, range, null, source);
  const taxonomy = taxonomies.data?.find((item) => item.id === chosen);

  if (!range || (taxonomies.isSuccess && chosen === null))
    return (
      <p className="muted">
        <Trans>No classification has been set up yet.</Trans>
      </p>
    );

  return (
    <Async
      query={income}
      isEmpty={(data) => data.total.events === 0}
      empty={
        <p className="muted">
          <Trans>No payments in this period.</Trans>
        </p>
      }
    >
      {(data) => (
        <AllocationBar
          items={data.nodes
            .filter((node) => node.summary.events > 0)
            .map((node) => ({
              key: node.key,
              label: bucketLabel(i18n, node),
              value: node.summary.net_base,
              weight: node.weight,
              slot: slotOfNode(taxonomy, node.key),
            }))}
          total={data.total.net_base}
          currency={data.base_currency}
          limit={Number(widget.cfg.count) || 8}
          tracks={false}
        />
      )}
    </Async>
  );
}

/** Where the portfolio stands against a target, as one bar per category. The plan itself
 * belongs to the rebalance screen; this only shows the drift that would call for it. */
export function DriftWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const target = useWidgetTarget(widget);
  const portfolio = usePortfolio();
  const plan = useRebalance(target.id, date, null, true, source);

  if (target.id === null)
    return (
      <p className="muted">
        <Trans>No target has been set up yet.</Trans>
      </p>
    );

  return (
    <Async query={plan}>
      {(data) => <DriftBars items={data.items} currency={portfolio.data?.base_currency ?? ""} />}
    </Async>
  );
}
