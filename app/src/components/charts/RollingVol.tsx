import { Trans, useLingui } from "@lingui/react/macro";
import { Chart, Grid, Marker, indexScale, valueAxis, type ChartHeight } from "./Chart";
import { CH, path } from "../../lib/plot";
import { axisFormat, formatDay, formatRate } from "../../lib/format";
import type { StatSeries } from "../../lib/types";

/** Shows annualized rolling volatility across the supplied window. */
export function RollingVol({
  series,
  windowDays,
  height = CH.h.sm,
}: {
  series: StatSeries;
  windowDays: number;
  height?: ChartHeight;
}) {
  const { t } = useLingui();
  if (series.dates.length === 0) {
    return (
      <p className="muted">
        <Trans>The series is shorter than the {windowDays}-trading-day window — nothing to compute.</Trans>
      </p>
    );
  }

  const last = series.values.length - 1;

  return (
    <Chart
      height={height}
      gutter={CH.gutter.pct}
      dates={series.dates}
      label={t`Rolling volatility, ${windowDays}-trading-day window`}
      readout={(i) => (
        <>
          <span className="chart__when">{formatDay(series.dates[i])}</span>
          <b className="num">{formatRate(series.values[i])}</b>
          <span>
            <Trans>annualized</Trans>
          </span>
        </>
      )}
    >
      {(frame, hover) => {
        const x = indexScale(series.dates.length, frame);
        const { y, ticks } = valueAxis(series.values, frame, true);
        return (
          <>
            <Grid frame={frame} ticks={ticks} y={y} format={axisFormat(ticks, { percent: true })} />
            <path className="chart__line" d={path(series.values.map((v, i) => [x(i), y(v)]))} />
            <Marker x={x(hover ?? last)} y={y(series.values[hover ?? last])} />
          </>
        );
      }}
    </Chart>
  );
}
