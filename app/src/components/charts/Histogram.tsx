import { Trans, useLingui } from "@lingui/react/macro";
import { Chart, type ChartHeight } from "./Chart";
import { CH, histogram, linearScale } from "../../lib/plot";
import { formatRate } from "../../lib/format";

/** Shows daily-return distribution with a width-dependent bin count. */
export function Histogram({ values, height = CH.h.sm }: { values: number[]; height?: ChartHeight }) {
  const { t } = useLingui();
  if (values.length === 0)
    return (
      <p className="muted">
        <Trans>No daily returns for this period.</Trans>
      </p>
    );

  return (
    <Chart
      height={height}
      gutter={CH.pad.left}
      label={t`Distribution of daily returns`}
      legend={[
        { label: t`losing days`, tone: "neg" },
        { label: t`winning days`, tone: "accent" },
      ]}
    >
      {(frame) => {
        const width = frame.right - frame.left;
        const bins = Math.min(Math.max(Math.round(width / 18), 5), 31);
        const cells = histogram(values, bins);
        if (cells.length === 0) return null;

        const from = cells[0].from;
        const to = cells[cells.length - 1].to;
        const x = linearScale([from, to], [frame.left, frame.right]);
        const peak = Math.max(...cells.map((c) => c.count), 1);
        const y = linearScale([0, peak], [frame.bottom, frame.top]);
        const barWidth = Math.max(width / bins - 2, 1);

        return (
          <>
            {cells.map((cell) => (
              <rect
                key={cell.from}
                className={cell.to <= 0 ? "chart__bar--neg" : "chart__bar"}
                x={x(cell.from) + 1}
                width={barWidth}
                y={y(cell.count)}
                height={Math.max(frame.bottom - y(cell.count), 0)}
                rx={CH.r.bar}
                data-tip={t`${formatRate(cell.from, 1)} … ${formatRate(cell.to, 1)}: ${cell.count} days`}
              />
            ))}

            {from < 0 && to > 0 && (
              <line className="chart__baseline" x1={x(0)} x2={x(0)} y1={frame.top} y2={frame.bottom} />
            )}

            <text className="chart__axis" x={frame.left} y={frame.height - 4}>
              {formatRate(from, 1)}
            </text>
            {from < 0 && to > 0 && (
              <text className="chart__axis" x={x(0)} y={frame.height - 4} textAnchor="middle">
                0
              </text>
            )}
            <text className="chart__axis" x={frame.right} y={frame.height - 4} textAnchor="end">
              {formatRate(to, 1)}
            </text>
          </>
        );
      }}
    </Chart>
  );
}
