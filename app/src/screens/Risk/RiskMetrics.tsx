import { useLingui } from "@lingui/react/macro";
import { Metric, Metrics, Num, Rate, Stat } from "../../components/ui";
import { formatDay } from "../../lib/format";
import type { RiskMetrics as Figures } from "../../lib/types";
import { dayCount } from "./days";

/** The headline figures of the period; `…` while the report is on its way. */
export function RiskMetrics({ metrics, riskFree }: { metrics: Figures | undefined; riskFree: string }) {
  const { t } = useLingui();
  const positiveDays = metrics ? Math.round(metrics.positive_days_share * metrics.days) : null;
  return (
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
  );
}
