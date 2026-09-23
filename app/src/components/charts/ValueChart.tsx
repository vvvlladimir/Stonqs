import { Trans, useLingui } from "@lingui/react/macro";
import { Area, Chart, Grid, Marker, type ChartHeight } from "./Chart";
import { CH, indexScale, path, toPlotNumber, valueAxis } from "../../lib/plot";
import { formatDay, formatMoney } from "../../lib/format";
import type { ValueSeries } from "../../lib/types";

/** Shows portfolio value and external flows on a shared time axis. */
interface Props {
  series: ValueSeries;
  currency: string;
  height?: ChartHeight;
}

/** Fixed lane height keeps flow bars visible without competing with the line. */
const LANE = 40;
/** Pitch of a flow column: daily bars would be thinner than the gap between them. */
const COLUMN = 9;

export function ValueChart({ series, currency, height = CH.h.md }: Props) {
  const { t } = useLingui();
  const dates = series.dates;
  const values = series.total_value_base.map(toPlotNumber);
  const flows = series.external_flow_base.map(toPlotNumber);

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
      lane={LANE}
      dates={dates}
      label={t`Portfolio value in ${currency}`}
      legend={[
        { label: t`Value`, tone: "accent" },
        { label: t`Deposits`, tone: "in" },
        { label: t`Withdrawals`, tone: "out" },
      ]}
      readout={(i) => (
        <>
          <span className="chart__when">{formatDay(dates[i])}</span>
          <b className="num">{formatMoney(series.total_value_base[i], currency)}</b>
          {flows[i] !== 0 && (
            <span className="num">
              <Trans>flow {formatMoney(series.external_flow_base[i], currency, { signed: true })}</Trans>
            </span>
          )}
        </>
      )}
    >
      {(frame, hover) => {
        const x = indexScale(dates.length, frame);
        const { y, ticks } = valueAxis(values, frame);
        const line = values.map((v, i): [number, number] => [x(i), y(v)]);

        // Positive flows rise above the lane center; withdrawals fall below it.
        const laneZero = (frame.laneTop + frame.laneBottom) / 2;
        const laneHalf = (frame.laneBottom - frame.laneTop) / 2;

        // One column can cover several days; deposits and withdrawals are summed apart
        // so a day with both does not cancel itself out of the chart.
        const span = frame.right - frame.left;
        const columns = Math.min(Math.max(Math.floor(span / COLUMN), 1), dates.length);
        const columnOf = (i: number) =>
          Math.min(Math.floor((i / Math.max(dates.length - 1, 1)) * columns), columns - 1);
        const deposits = new Array<number>(columns).fill(0);
        const withdrawals = new Array<number>(columns).fill(0);
        flows.forEach((flow, i) => {
          if (flow > 0) deposits[columnOf(i)] += flow;
          else if (flow < 0) withdrawals[columnOf(i)] -= flow;
        });

        const pitch = span / columns;
        const barWidth = Math.max(Math.min(pitch - 2, 10), 2);
        const columnX = (c: number) => frame.left + (c + 0.5) * pitch;

        // A single large transfer would flatten every regular one under a linear scale,
        // so the lane compares flows by square root: this lane answers "when and roughly
        // how much", the exact amount is in the readout.
        const peak = Math.max(...deposits, ...withdrawals, 1);
        const size = (value: number) => Math.max(Math.sqrt(value / peak) * laneHalf, 3);
        const lit = hover === null ? null : columnOf(hover);

        return (
          <>
            <Grid frame={frame} ticks={ticks} y={y} />
            <Area points={line} baseline={frame.bottom} />
            <path className="chart__line" d={path(line)} />

            <line className="chart__lane" x1={frame.left} x2={frame.right} y1={laneZero} y2={laneZero} />
            {deposits.map((value, c) => {
              if (value === 0) return null;
              const h = size(value);
              return (
                <rect
                  key={`in-${c}`}
                  className="chart__flow--in"
                  data-on={lit === c ? "1" : undefined}
                  x={columnX(c) - barWidth / 2}
                  width={barWidth}
                  y={laneZero - h}
                  height={h}
                  rx={CH.r.bar}
                />
              );
            })}
            {withdrawals.map((value, c) => {
              if (value === 0) return null;
              const h = size(value);
              return (
                <rect
                  key={`out-${c}`}
                  className="chart__flow--out"
                  data-on={lit === c ? "1" : undefined}
                  x={columnX(c) - barWidth / 2}
                  width={barWidth}
                  y={laneZero}
                  height={h}
                  rx={CH.r.bar}
                />
              );
            })}

            {hover !== null && <Marker x={x(hover)} y={y(values[hover])} />}
          </>
        );
      }}
    </Chart>
  );
}
