import { Trans, useLingui } from "@lingui/react/macro";
import { Area, Chart, Grid, Marker, type ChartHeight } from "./Chart";
import { CH, indexScale, path, valueAxis } from "../../lib/plot";
import { axisFormat, formatDay, formatRate } from "../../lib/format";
import type { StatSeries } from "../../lib/types";

/** Shows drawdown as a downward area from the zero baseline. */
export function Underwater({ series, height = CH.h.sm }: { series: StatSeries; height?: ChartHeight }) {
  const { t } = useLingui();
  if (series.dates.length === 0)
    return (
      <p className="muted">
        <Trans>No data for this period.</Trans>
      </p>
    );

  return (
    <Chart
      height={height}
      gutter={CH.gutter.pct}
      dates={series.dates}
      label={t`Drawdown from the peak, by day`}
      readout={(i) => (
        <>
          <span className="chart__when">{formatDay(series.dates[i])}</span>
          <b className="num">{formatRate(series.values[i], 2)}</b>
          <span>
            <Trans>from the peak</Trans>
          </span>
        </>
      )}
    >
      {(frame, hover) => {
        const x = indexScale(series.dates.length, frame);
        const { y, ticks } = valueAxis(series.values, frame, true);
        const line = series.values.map((v, i): [number, number] => [x(i), y(v)]);
        return (
          <>
            <Grid frame={frame} ticks={ticks} y={y} format={axisFormat(ticks, { percent: true })} />
            <Area points={line} baseline={y(0)} tone="neg" />
            <path className="chart__line chart__line--neg" d={path(line)} />
            <line className="chart__baseline" x1={frame.left} x2={frame.right} y1={y(0)} y2={y(0)} />
            {hover !== null && <Marker x={x(hover)} y={y(series.values[hover])} />}
          </>
        );
      }}
    </Chart>
  );
}
