import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import {
  useBenchmarkCompare,
  usePerformance,
  usePositionReturns,
  useRealPerformance,
  useSecurities,
} from "../../lib/queries";
import { Calendar, ValueChart } from "../../components/charts";
import { Page } from "../../components/Page";
import { Async, Choice, Empty, Panel, Pending, QueryError } from "../../components/ui";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { formatDay, formatPercent } from "../../lib/format";
import { CalculationSheet } from "./CalculationSheet";
import { PerformanceMetrics } from "./Metrics";
import { PositionReturns } from "./PositionReturns";
import { byContribution, monthCell } from "./model";
import { useAsOf } from "../../lib/asOf";

export function Performance() {
  const { t } = useLingui();
  const asOf = useAsOf().date;
  const [period, setPeriod] = useState<PeriodId>("SINCE_INCEPTION");
  const [benchmarkId, setBenchmarkId] = useState("");

  const ranges = usePeriodRanges(asOf);
  const range = pickRange(ranges.data, period);

  const performance = usePerformance(range);
  const securities = useSecurities();
  // Core computes excess return in Decimal; keep arithmetic out of the UI.
  const comparison = useBenchmarkCompare(benchmarkId, range);
  // Fetch all position returns together to avoid one history pass per security.
  const positions = usePositionReturns(range);
  // Null until the portfolio names a price-index region; the strip then grows two tiles.
  const real = useRealPerformance(range);

  if (ranges.isError) return <QueryError error={ranges.error} />;
  if (ranges.isPending) return <Pending />;
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions — there is no return to compute.</Trans>
      </p>
    );

  const summary = performance.data;
  const currency = summary?.base_currency ?? "";
  const benchmarkLabel = securities.data?.find((s) => s.id === benchmarkId)?.symbol;

  return (
    <Page
      archetype="analysis"
      title={t`Performance`}
      asOf={`${formatDay(range.from)} — ${formatDay(range.to)}`}
      controls={
        <>
          <Choice
            label={t`Benchmark instrument`}
            placeholder={t`No benchmark`}
            value={benchmarkId}
            onChange={setBenchmarkId}
            options={(securities.data ?? []).map((sec) => ({
              value: sec.id,
              label: `${sec.symbol} — ${sec.name}`,
            }))}
          />
          <PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />
        </>
      }
      metrics={
        <PerformanceMetrics
          data={summary}
          benchmarkId={benchmarkId}
          benchmarkLabel={benchmarkLabel}
          comparison={comparison.data}
          // Dates are ISO, so a string compare is a date compare.
          since={comparison.data && comparison.data.from > range.from ? comparison.data.from : undefined}
          real={real.data}
        />
      }
      banner={
        // A benchmark with no prices in the window used to fail on its own panel; the two tiles
        // it feeds are all that is left of it, so the failure belongs to the page.
        performance.isError ? (
          <QueryError error={performance.error} />
        ) : comparison.isError ? (
          <QueryError error={comparison.error} />
        ) : undefined
      }
    >
      <Panel
        chart
        title={t`Value and flows`}
        info={t`Deposits and withdrawals run under the chart, so a deposit is not mistaken for a gain.`}
      >
        <Async query={performance}>
          {(data) => <ValueChart series={data.series} currency={data.base_currency} height="fill" />}
        </Async>
      </Panel>

      <Panel
        title={t`Return by month`}
        info={t`Time-weighted return of each month, so money paid in or out does not count as a gain or a loss.`}
      >
        <Async query={performance}>
          {(data) => (
            <Calendar
              cells={data.monthly_returns.map(monthCell)}
              totals={data.annual_returns.map((year) => ({
                year: Number(year.from.slice(0, 4)),
                text: formatPercent(year.twr, { digits: 0, signed: true }),
              }))}
            />
          )}
        </Async>
      </Panel>

      <CalculationSheet range={range} currency={currency} />

      <Panel
        title={t`Return by position`}
        info={t`What each instrument earned over the period and how much of the portfolio return it accounts for.`}
        table
      >
        <Async
          query={positions}
          empty={
            <Empty title={t`No positions were open in this period`}>
              <Trans>
                Pick a wider period or add transactions — contribution counts the instruments held inside the
                period.
              </Trans>
            </Empty>
          }
        >
          {(data) => <PositionReturns rows={byContribution(data)} currency={currency} />}
        </Async>
      </Panel>
    </Page>
  );
}
