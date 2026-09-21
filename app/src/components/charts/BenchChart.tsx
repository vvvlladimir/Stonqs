import { Fragment } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Chart, Grid, Marker, indexScale, valueAxis, type ChartHeight } from "./Chart";
import { CH, path, slotVar, tickStep, toPlotNumber } from "../../lib/plot";
import { formatDay, formatPercent } from "../../lib/format";
import type { GrowthSeries } from "../../lib/types";

/** One reference line: its name, its series (absent while loading or without prices) and its
 * palette slot, which is also its identity — two instruments may share a symbol. */
export interface BenchLine {
  label: string;
  series?: GrowthSeries | null;
  slot: number;
}

/** Compares core-aligned unit-growth series without cash-flow distortion. */
interface Props {
  portfolio: GrowthSeries;
  benchmarks: BenchLine[];
  height?: ChartHeight;
}

export function BenchChart({ portfolio, benchmarks, height = CH.h.md }: Props) {
  const { t } = useLingui();
  const dates = portfolio.dates;
  const mine = portfolio.values.map(toPlotNumber);
  // Aligned by date, not by index: an instrument younger than the period comes back with a
  // shorter series, and its line has to start where its prices do rather than at the left edge.
  const lines = benchmarks
    .map((line) => {
      const growthOn = new Map(
        line.series?.dates.map((day, i) => [day, toPlotNumber(line.series!.values[i])]),
      );
      return { ...line, values: dates.map((day) => growthOn.get(day) ?? null) };
    })
    .filter((line) => line.values.some((value) => value !== null));
  const drawn = lines.flatMap((line) => line.values.filter((value): value is number => value !== null));

  if (dates.length === 0)
    return (
      <p className="muted">
        <Trans>No data for this period.</Trans>
      </p>
    );

  const names = lines.map((line) => line.label).join(", ");
  return (
    <Chart
      height={height}
      gutter={CH.gutter.pct}
      dates={dates}
      label={lines.length > 0 ? t`Portfolio growth against ${names}` : t`Portfolio growth`}
      legend={[
        { label: t`Portfolio`, tone: "accent" },
        ...lines.map((line) => ({ label: line.label, tone: "ref" as const, slot: line.slot })),
      ]}
      readout={(i) => (
        <>
          <span className="chart__when">{formatDay(dates[i])}</span>
          <b className="num">{growth(mine[i], 2)}</b>
          <span>
            <Trans>portfolio</Trans>
          </span>
          {lines.map(
            (line) =>
              line.values[i] !== null && (
                <Fragment key={line.slot}>
                  <span className="num">{growth(line.values[i], 2)}</span>
                  <span>{line.label}</span>
                </Fragment>
              ),
          )}
        </>
      )}
    >
      {(frame, hover) => {
        const x = indexScale(dates.length, frame);
        const { y, ticks } = valueAxis([...mine, ...drawn], frame);
        // Growth is a multiple, so the axis prints it as a gain: a step under a percent
        // needs a decimal or two gridlines would read the same.
        const digits = tickStep(ticks) >= 0.01 ? 0 : 1;
        return (
          <>
            <Grid frame={frame} ticks={ticks} y={y} format={(v) => growth(v, digits)} />
            {/* Baseline at 1.0 represents the break-even threshold. */}
            <line className="chart__baseline" x1={frame.left} x2={frame.right} y1={y(1)} y2={y(1)} />
            {lines.map((line) => (
              <path
                key={line.slot}
                className="chart__ref"
                style={slotVar(line.slot)}
                d={path(
                  line.values.flatMap((v, i) => (v === null ? [] : [[x(i), y(v)] as [number, number]])),
                )}
              />
            ))}
            <path className="chart__line" d={path(mine.map((v, i) => [x(i), y(v)]))} />
            {hover !== null && (
              <>
                {lines.map((line) => {
                  const value = line.values[hover];
                  return (
                    value !== null && (
                      <Marker key={line.slot} x={x(hover)} y={y(value)} tone="ref" slot={line.slot} />
                    )
                  );
                })}
                <Marker x={x(hover)} y={y(mine[hover])} />
              </>
            )}
          </>
        );
      }}
    </Chart>
  );
}

function growth(value: number, digits = 0): string {
  return formatPercent(String(value - 1), { digits, signed: true });
}
