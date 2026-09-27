import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Histogram, RollingVol, Underwater } from "../../components/charts";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { Page } from "../../components/Page";
import { Async, Choice, Panel, Pending, QueryError } from "../../components/ui";
import { useAsOf } from "../../lib/asOf";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import { useRisk } from "../../lib/queries";
import { DrawdownTable } from "./DrawdownTable";
import { RiskMetrics } from "./RiskMetrics";

const WINDOW_DAYS = [21, 63, 126, 252];

export function Risk() {
  const { t } = useLingui();
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

  return (
    <Page
      archetype="analysis"
      title={t`Risk`}
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
      metrics={<RiskMetrics metrics={metrics} riskFree={riskFree} />}
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
        <Async query={report}>{(risk) => <DrawdownTable episodes={risk.episodes} />}</Async>
      </Panel>
    </Page>
  );
}
