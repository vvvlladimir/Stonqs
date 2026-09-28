import { Trans } from "@lingui/react/macro";
import { Async } from "../../../components/ui";
import {
  BenchChart,
  Calendar,
  Underwater,
  ValueChart,
  Contributions,
  returnCell,
} from "../../../components/charts";
import {
  useBenchmarks,
  useInflationSeries,
  useInflationStatus,
  usePerformance,
  usePositionReturns,
  useSecurities,
} from "../../../lib/queries";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { formatPercent, formatRegion } from "../../../lib/format";
import { slotFor } from "../../../lib/plot";
import { benchmarksOf, inflationOf, periodOf, useWidgetRisk, type WidgetProps, sourceOf } from "./model";

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
