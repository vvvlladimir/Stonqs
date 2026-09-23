import type { I18n } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { usePeriodRanges, pickRange, type PeriodId } from "../lib/periods";
import { useRisk } from "../lib/queries";
import { Histogram, RollingVol, Underwater } from "../components/charts";
import { Page } from "../components/Page";
import { PeriodControl } from "../components/domain/PeriodControl";
import {
  Async,
  Choice,
  DataTable,
  Empty,
  Metric,
  Metrics,
  Num,
  Panel,
  Pending,
  QueryError,
  Rate,
  Stat,
} from "../components/ui";
import { formatDay, formatMonth } from "../lib/format";
import type { Drawdown } from "../lib/types";
import { useAsOf } from "../lib/asOf";

const WINDOW_DAYS = [21, 63, 126, 252];

export function Risk() {
  const { t, i18n } = useLingui();
  const asOf = useAsOf().date;
  const [period, setPeriod] = useState<PeriodId>("SINCE_INCEPTION");
  const [window, setWindow] = useState("63");
  const riskFree = "2";

  const ranges = usePeriodRanges(asOf);
  const range = pickRange(ranges.data, period);

  const report = useRisk(range, riskFree, window);

  if (ranges.isError) return <QueryError error={ranges.error} />;
  if (ranges.isPending) return <Pending />;
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions — there is no risk to compute.</Trans>
      </p>
    );

  const data = report.data;
  const metrics = data?.metrics;
  const positiveDays = metrics && data ? Math.round(metrics.positive_days_share * metrics.days) : null;

  return (
    <Page
      archetype="analysis"
      title={t`Risk`}
      asOf={`${formatDay(range.from)} — ${formatDay(range.to)}`}
      controls={
        <>
          <Choice
            label={t`Window`}
            value={window}
            onChange={setWindow}
            options={WINDOW_DAYS.map((days) => ({
              value: String(days),
              label: t`Window: ${days} days`,
            }))}
          />
          <PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />
        </>
      }
      metrics={
        <Metrics>
          <Metric
            label={t`Volatility (annualized)`}
            value={metrics ? <Rate value={metrics.volatility} /> : "…"}
            hint={t`√252 over ${metrics?.days ?? "…"} trading days`}
            tip={t`Standard deviation of daily returns, annualised.`}
          />
          <Metric
            label={t`Sharpe`}
            value={metrics ? <Stat value={metrics.sharpe} /> : "…"}
            hint={
              metrics?.sharpe === null
                ? t`zero volatility — nothing to divide by`
                : t`risk-free ${riskFree || "0"} %`
            }
            tip={t`Return above the risk-free rate per unit of volatility. Above 1 is good.`}
          />
          <Metric
            label={t`Maximum drawdown`}
            value={metrics?.max_drawdown ? <Rate value={metrics.max_drawdown.depth} /> : t`no drawdowns`}
            tone={metrics?.max_drawdown ? "negative" : "neutral"}
            hint={
              metrics?.max_drawdown
                ? metrics.max_drawdown_days == null
                  ? formatDay(metrics.max_drawdown.trough)
                  : t`${formatDay(metrics.max_drawdown.trough)} · ${dayCount(metrics.max_drawdown_days)}`
                : undefined
            }
            tip={t`The deepest fall from a peak, measured on returns rather than value.`}
          />
          <Metric
            label={t`Current drawdown`}
            value={metrics ? <Rate value={metrics.current_drawdown} /> : "…"}
            tone={metrics && metrics.current_drawdown < 0 ? "negative" : "neutral"}
            hint={
              metrics?.current_drawdown_since
                ? t`under the peak of ${formatDay(metrics.current_drawdown_since)}`
                : t`at its peak`
            }
            tip={t`How far below the last peak the period ends.`}
          />
          <Metric
            label={t`Longest drawdown`}
            value={
              metrics?.longest_drawdown_days != null ? (
                <Num>{dayCount(metrics.longest_drawdown_days)}</Num>
              ) : (
                t`no drawdowns`
              )
            }
            hint={
              metrics?.longest_drawdown
                ? metrics.longest_drawdown.recovered
                  ? t`from ${formatDay(metrics.longest_drawdown.peak)}, recovered`
                  : t`from ${formatDay(metrics.longest_drawdown.peak)}, still open`
                : undefined
            }
            tip={t`Days between a peak and the return to it.`}
          />
          <Metric
            label={t`Winning days`}
            value={metrics ? <Rate value={metrics.positive_days_share} /> : "…"}
            hint={metrics && positiveDays !== null ? t`${positiveDays} of ${metrics.days}` : undefined}
            tip={t`Share of trading days that closed higher.`}
          />
          <Metric
            label={t`Semi-deviation`}
            value={metrics ? <Rate value={metrics.semi_deviation} /> : "…"}
            hint={t`losing days only`}
            tip={t`How much of the swing the falling days account for.`}
          />
          <Metric
            label={t`Return (annualized)`}
            value={metrics ? <Rate value={metrics.annualized_return} /> : "…"}
            tone={metrics ? (metrics.annualized_return >= 0 ? "positive" : "negative") : "neutral"}
            hint={t`period TWR, per year`}
            tip={t`Time-weighted return as a yearly rate.`}
          />
          <Metric
            label={t`Best day`}
            value={metrics?.best_day ? <Rate value={metrics.best_day[1]} digits={2} /> : "…"}
            tone="positive"
            hint={metrics?.best_day ? formatDay(metrics.best_day[0]) : undefined}
            tip={t`The strongest trading day.`}
          />
          <Metric
            label={t`Worst day`}
            value={metrics?.worst_day ? <Rate value={metrics.worst_day[1]} digits={2} /> : "…"}
            tone="negative"
            hint={metrics?.worst_day ? formatDay(metrics.worst_day[0]) : undefined}
            tip={t`The weakest trading day.`}
          />
        </Metrics>
      }
      banner={report.isError ? <QueryError error={report.error} /> : undefined}
    >
      <Panel
        chart
        title={t`Drawdown from the peak`}
        info={t`How far the return has fallen below its last high; deposits and withdrawals do not move it.`}
      >
        <Async query={report}>{(risk) => <Underwater series={risk.drawdown} height="fill" />}</Async>
      </Panel>

      <div className="grid-2 stack">
        <Panel
          chart
          title={t`Rolling volatility`}
          info={
            data
              ? t`Annualized volatility over a sliding window of ${data.window_days} trading days.`
              : t`Annualized volatility over a sliding window of trading days.`
          }
        >
          <Async query={report}>
            {(risk) => (
              <RollingVol series={risk.rolling_volatility} windowDays={risk.window_days} height="fill" />
            )}
          </Async>
        </Panel>

        <Panel
          chart
          title={t`Distribution of daily returns`}
          info={
            metrics
              ? t`How often each size of daily return occurred over ${metrics.days} trading days.`
              : t`How often each size of daily return occurred in the period.`
          }
        >
          <Async query={report}>{(risk) => <Histogram values={risk.returns.values} height="fill" />}</Async>
        </Panel>
      </div>

      <Panel
        title={t`Deepest drawdowns`}
        info={t`The worst falls from a high to a low, deepest first rather than latest first.`}
        table
      >
        <Async query={report}>
          {(risk) => (
            <DataTable
              variant="rows"
              rows={byDepth(risk.episodes)}
              rowKey={(episode) => `${episode.peak}-${episode.trough}`}
              empty={
                <Empty title={t`No drawdowns`}>
                  <Trans>Over the selected period the value never fell below a previous peak.</Trans>
                </Empty>
              }
              columns={[
                {
                  key: "span",
                  sort: (episode) => episode.peak,
                  header: t`Period`,
                  align: "left",
                  cell: (episode) => span(i18n, episode),
                  card: "title",
                },
                {
                  key: "depth",
                  sort: (episode) => episode.depth,
                  header: t`Depth`,
                  className: "neg",
                  card: "value",
                  cell: (episode) => <Rate value={episode.depth} digits={2} />,
                },
                {
                  key: "fall",
                  sort: (episode) => dayDelta(episode.peak, episode.trough),
                  header: (
                    <span data-tip={t`Days from peak to trough.`}>
                      <Trans>Fall</Trans>
                    </span>
                  ),
                  factLabel: t`fall`,
                  cell: (episode) => <Num>{days(episode.peak, episode.trough)}</Num>,
                },
                {
                  key: "recovery",
                  sort: (episode) => (episode.recovered ? dayDelta(episode.trough, episode.recovered) : null),
                  header: (
                    <span data-tip={t`Days from the trough back to the peak.`}>
                      <Trans>Recovery</Trans>
                    </span>
                  ),
                  factLabel: t`recovery`,
                  cell: (episode) =>
                    episode.recovered ? (
                      <Num>{days(episode.trough, episode.recovered)}</Num>
                    ) : (
                      t`not recovered`
                    ),
                },
                {
                  key: "total",
                  sort: (episode) => (episode.recovered ? dayDelta(episode.peak, episode.recovered) : null),
                  header: t`Total`,
                  card: "none",
                  cell: (episode) =>
                    episode.recovered ? <Num>{days(episode.peak, episode.recovered)}</Num> : "—",
                },
              ]}
            />
          )}
        </Async>
      </Panel>
    </Page>
  );
}

/** Sort deeper drawdowns first; `depth` is negative. */
function byDepth(episodes: Drawdown[]): Drawdown[] {
  return [...episodes].sort((a, b) => a.depth - b.depth);
}

/** An open drawdown is reported explicitly rather than as missing data. */
function span(i18n: I18n, episode: Drawdown): string {
  const [year, month] = episode.peak.split("-").map(Number);
  const from = formatMonth(year, month);
  if (!episode.recovered) return i18n._(msg`${from} — now`);
  const [ry, rm] = episode.recovered.split("-").map(Number);
  return `${from} — ${formatMonth(ry, rm)}`;
}

/** Compute calendar-day duration from ISO dates without timezone shifts. */
function dayDelta(from: string, to: string): number {
  const [fy, fm, fd] = from.split("-").map(Number);
  const [ty, tm, td] = to.split("-").map(Number);
  return (Date.UTC(ty, tm - 1, td) - Date.UTC(fy, fm - 1, fd)) / 86_400_000;
}

function days(from: string, to: string): string {
  return dayCount(dayDelta(from, to));
}

/** A duration the core already counted; only the wording is the interface's business. */
function dayCount(value: number): string {
  return plural(value, { one: "# day", other: "# days" });
}
