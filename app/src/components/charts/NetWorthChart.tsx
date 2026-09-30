import { Trans, useLingui } from "@lingui/react/macro";
import { Chart, Grid, Marker, type ChartHeight } from "./Chart";
import { CH, indexScale, path, toPlotNumber, valueAxis } from "../../lib/plot";
import { axisFormat, formatDay, formatMoney } from "../../lib/format";
import type { NetWorthSeries } from "../../lib/types";

/** Net worth against what is invested. The asset side steps between valuations, so the gap
 *  between the two lines changes on the days somebody wrote a figure, not every day. */
export function NetWorthChart({
  series,
  currency,
  height = CH.h.md,
}: {
  series: NetWorthSeries;
  currency: string;
  height?: ChartHeight;
}) {
  const { t } = useLingui();
  const dates = series.points.map((p) => p.date);
  const net = series.points.map((p) => toPlotNumber(p.net_base));
  const invested = series.points.map((p) => toPlotNumber(p.investments_base));

  if (dates.length === 0)
    return (
      <p className="muted">
        <Trans>No data for this period.</Trans>
      </p>
    );

  return (
    <Chart
      height={height}
      gutter={CH.gutter.money}
      dates={dates}
      label={t`Net worth and invested value in ${currency}`}
      legend={[
        { label: t`Net worth`, tone: "accent" },
        { label: t`Invested`, tone: "ref" },
      ]}
      readout={(i) => (
        <>
          <span className="chart__when">{formatDay(dates[i])}</span>
          <b className="num">{formatMoney(series.points[i].net_base, currency)}</b>
          <span className="num">
            <Trans>invested {formatMoney(series.points[i].investments_base, currency)}</Trans>
          </span>
        </>
      )}
    >
      {(frame, hover) => {
        const x = indexScale(dates.length, frame);
        // Zero is on the axis: net worth can be negative, and a line that never shows the
        // crossing would hide the one fact that matters about a negative one.
        const { y, ticks } = valueAxis([...net, ...invested], frame, true);
        const netLine = net.map((v, i): [number, number] => [x(i), y(v)]);
        const investedLine = invested.map((v, i): [number, number] => [x(i), y(v)]);

        return (
          <>
            <Grid frame={frame} ticks={ticks} y={y} format={axisFormat(ticks)} />
            <path className="chart__line chart__line--ref" d={path(investedLine)} />
            <path className="chart__line" d={path(netLine)} />
            <line className="chart__baseline" x1={frame.left} x2={frame.right} y1={y(0)} y2={y(0)} />
            {hover !== null && <Marker x={x(hover)} y={y(net[hover])} />}
          </>
        );
      }}
    </Chart>
  );
}
