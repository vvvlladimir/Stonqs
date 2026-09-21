import { useLingui } from "@lingui/react/macro";
import { Metric, Metrics as MetricStrip, Money, Percent } from "../../components/ui";
import { formatDay, formatRegion, signOf } from "../../lib/format";
import type { BenchmarkComparison, PerformanceData, RealPerformance } from "../../lib/types";

interface Props {
  data?: PerformanceData;
  /** Empty until the user picks one with the header's benchmark control. */
  benchmarkId: string;
  benchmarkLabel?: string;
  comparison?: BenchmarkComparison;
  /** Set when the benchmark is younger than the period: the day the comparison really opens. */
  since?: string;
  /** Absent until the portfolio names a price-index region; then the same period, deflated. */
  real?: RealPerformance | null;
}

/**
 * The period read twice over: what the portfolio returned, and what the money did. A rise in
 * value is not a result while half of it was deposited, so `delta` sits beside `absolute
 * change` rather than replacing it.
 */
export function PerformanceMetrics({ data, benchmarkId, benchmarkLabel, comparison, since, real }: Props) {
  const { t } = useLingui();
  const currency = data?.base_currency ?? "";
  const summary = data?.summary;
  const noBenchmark = benchmarkId === "";

  return (
    <MetricStrip>
      <Metric
        label={t`TWR (portfolio return)`}
        value={data ? <Percent value={data.twr} signed tone={false} /> : "…"}
        tone={data ? signOf(data.twr) : "neutral"}
        hint={
          data?.twr_annualized ? (
            <>
              <Percent value={data.twr_annualized} signed tone={false} /> {t`a year`}
            </>
          ) : (
            t`free of deposits`
          )
        }
        tip={t`What the portfolio did on its own, regardless of when money was added.`}
      />
      <Metric
        label={t`XIRR (investor return)`}
        value={data?.xirr ? <Percent value={data.xirr} signed tone={false} /> : t`— did not converge`}
        tone={data?.xirr ? signOf(data.xirr) : "neutral"}
        hint={
          data?.xirr
            ? t`including the dates of deposits`
            : t`The flows never change sign — the equation has no root`
        }
        tip={t`Return that accounts for the dates money went in and out.`}
      />
      {real && (
        <>
          <Metric
            label={t`Real TWR`}
            value={<Percent value={real.twr.real} signed tone={false} />}
            tone={signOf(real.twr.real)}
            hint={
              <>
                {formatRegion(real.twr.region)} {t`prices`}{" "}
                <Percent value={real.twr.inflation} signed tone={false} />
              </>
            }
            tip={t`The portfolio return with inflation taken out, through ${formatDay(real.twr.to)}.`}
          />
          <Metric
            label={t`Real XIRR`}
            value={real.xirr ? <Percent value={real.xirr} signed tone={false} /> : t`— did not converge`}
            tone={real.xirr ? signOf(real.xirr) : "neutral"}
            hint={t`each deposit at its own price level`}
            tip={t`Investor return with inflation taken out, flow by flow.`}
          />
        </>
      )}
      <Metric
        label={t`Earned over the period`}
        value={summary ? <Money value={summary.delta_base} currency={currency} signed /> : "…"}
        tone={summary ? signOf(summary.delta_base) : "neutral"}
        hint={
          summary ? (
            <>
              {t`value changed by`} <Money value={summary.absolute_change_base} currency={currency} signed />
            </>
          ) : undefined
        }
        tip={t`The change in value with deposits and withdrawals taken out.`}
      />
      <Metric
        label={t`Invested capital`}
        value={summary ? <Money value={summary.invested_capital_base} currency={currency} /> : "…"}
        hint={
          summary ? (
            <>
              {t`added`} <Money value={summary.net_flow_base} currency={currency} signed />
            </>
          ) : undefined
        }
        tip={t`Opening value plus everything paid in since.`}
      />
      <Metric
        label={t`Fee rate`}
        value={data?.fee_rate ? <Percent value={data.fee_rate} digits={2} /> : "—"}
        hint={data ? <Money value={data.costs.fees_base} currency={currency} /> : undefined}
        tip={t`All fees against the capital at work, trade commissions included.`}
      />
      <Metric
        label={t`Tax rate`}
        value={data?.tax_rate ? <Percent value={data.tax_rate} digits={2} /> : "—"}
        hint={data ? <Money value={data.costs.taxes_base} currency={currency} /> : undefined}
        tip={t`Tax against the capital at work, dividend withholding included.`}
      />
      <Metric
        label={t`Turnover`}
        value={data?.turnover_rate ? <Percent value={data.turnover_rate} digits={1} /> : "—"}
        hint={
          data ? (
            <>
              {t`traded`} <Money value={data.volume.volume_base} currency={currency} />
            </>
          ) : undefined
        }
        tip={t`Everything bought and sold in the period against the capital at work.`}
      />
      <Metric
        label={t`Period high`}
        value={data?.peak ? <Money value={data.peak.value} currency={currency} /> : "—"}
        tone={data?.peak?.distance ? signOf(data.peak.distance) : "neutral"}
        hint={
          data?.peak ? (
            data.peak.distance ? (
              <>
                <Percent value={data.peak.distance} signed tone={false} />{" "}
                {t`below the high of ${formatDay(data.peak.date)}`}
              </>
            ) : (
              t`reached on ${formatDay(data.peak.date)}`
            )
          ) : undefined
        }
        tip={t`The highest value inside the period, and how far under it the period ends.`}
      />
      <Metric
        label={t`Benchmark`}
        value={
          noBenchmark ? (
            t`— none chosen`
          ) : comparison ? (
            <Percent value={comparison.benchmark_twr} signed tone={false} />
          ) : (
            "…"
          )
        }
        tone={comparison && !noBenchmark ? signOf(comparison.benchmark_twr) : "neutral"}
        hint={
          noBenchmark
            ? t`chosen in the header`
            : since
              ? t`${benchmarkLabel ?? ""} — since ${formatDay(since)}`
              : benchmarkLabel
        }
        tip={t`Time-weighted return of the benchmark over the window the two share.`}
      />
      <Metric
        label={t`Excess`}
        value={
          noBenchmark ? "—" : comparison ? <Percent value={comparison.excess} signed tone={false} /> : "…"
        }
        tone={comparison && !noBenchmark ? signOf(comparison.excess) : "neutral"}
        hint={t`portfolio minus benchmark`}
        tip={t`The difference of the two time-weighted returns.`}
      />
    </MetricStrip>
  );
}
