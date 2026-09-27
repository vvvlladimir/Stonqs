import { Trans, useLingui } from "@lingui/react/macro";
import { QUOTE_WINDOW_DAYS, useQuoteWindow } from "../../../lib/queries";
import { Area, Chart, Grid, Marker, indexScale, valueAxis } from "../../charts";
import { Money, Panel, Rate, Skeleton } from "../../ui";
import { CH, path, toPlotNumber } from "../../../lib/plot";
import { formatDay } from "../../../lib/format";
import type { Quote } from "../../../lib/types";

/** The last quote window and how far it moved. */
export function QuotePanel({ securityId, symbol }: { securityId: string; symbol: string }) {
  const { t } = useLingui();
  const quotes = useQuoteWindow(securityId);
  return (
    <Panel
      title={t`90 days`}
      tools={
        quotes.data && quotes.data.length > 1 ? (
          <span className="panel__note">
            <Rate
              value={change(quotes.data)}
              digits={1}
              className={change(quotes.data) < 0 ? "neg" : "pos"}
            />
          </span>
        ) : undefined
      }
    >
      {quotes.isPending ? (
        <Skeleton h={`${CH.h.sm}px`} />
      ) : (
        <PriceChart quotes={quotes.data ?? []} symbol={symbol} />
      )}
    </Panel>
  );
}

/** Renders the last 90 days of cached quote history. */
function PriceChart({ quotes, symbol }: { quotes: Quote[]; symbol: string }) {
  const { t } = useLingui();
  if (quotes.length < 2)
    return (
      <p className="muted">
        <Trans>No quotes for this window.</Trans>
      </p>
    );

  const dates = quotes.map((q) => q.date);
  const values = quotes.map((q) => toPlotNumber(q.close));
  const currency = quotes[quotes.length - 1].currency;

  return (
    <Chart
      height={CH.h.sm}
      dates={dates}
      label={t`${symbol} price over ${QUOTE_WINDOW_DAYS} days`}
      readout={(i) => (
        <>
          <span className="chart__when">{formatDay(dates[i])}</span>
          <b>
            <Money value={quotes[i].close} currency={currency} />
          </b>
        </>
      )}
    >
      {(frame, hover) => {
        const x = indexScale(dates.length, frame);
        const { y, ticks } = valueAxis(values, frame);
        const line = values.map((v, i): [number, number] => [x(i), y(v)]);
        const at = hover ?? dates.length - 1;
        return (
          <>
            <Grid frame={frame} ticks={ticks} y={y} />
            <Area points={line} baseline={frame.bottom} />
            <path className="chart__line" d={path(line)} />
            <Marker x={x(at)} y={y(values[at])} />
          </>
        );
      }}
    </Chart>
  );
}

/** Calculates the quote change over the displayed window. */
function change(quotes: Quote[]): number {
  const first = toPlotNumber(quotes[0].close);
  const last = toPlotNumber(quotes[quotes.length - 1].close);
  return first === 0 ? 0 : (last - first) / first;
}
