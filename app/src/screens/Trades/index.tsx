import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import { useTrades } from "../../lib/queries";
import { Page } from "../../components/Page";
import { Async, Empty, Panel, Pending, QueryError } from "../../components/ui";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { formatDay } from "../../lib/format";
import { TradeMetrics } from "./TradeMetrics";
import { TradesTable } from "./TradesTable";
import { useAsOf } from "../../lib/asOf";

/**
 * A trade is one purchase and the sales that emptied it — not an instrument and not a
 * transaction. The period selects which trades were *closed*; what is still held is shown
 * as of its end, because an open trade has no date to fall inside a window.
 */
export function Trades() {
  const { t } = useLingui();
  const asOf = useAsOf().date;
  const [period, setPeriod] = useState<PeriodId>("SINCE_INCEPTION");

  const ranges = usePeriodRanges(asOf);
  const range = pickRange(ranges.data, period);
  const trades = useTrades(range);

  if (ranges.isError) return <QueryError error={ranges.error} />;
  if (ranges.isPending) return <Pending />;
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions — there are no trades to show.</Trans>
      </p>
    );

  const currency = trades.data?.base_currency ?? "";

  return (
    <Page
      archetype="analysis"
      title={t`Trades`}
      asOf={`${formatDay(range.from)} — ${formatDay(range.to)}`}
      controls={<PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />}
      metrics={<TradeMetrics data={trades.data} />}
      banner={trades.isError ? <QueryError error={trades.error} /> : undefined}
    >
      <Panel
        title={t`Closed in the period`}
        info={t`Entry includes the purchase commission and exit is net of the sale's fees and tax; IRR also weighs how long the money was held.`}
        table
      >
        <Async
          query={trades}
          isEmpty={(data) => data.closed.length === 0}
          empty={
            <Empty title={t`Nothing was closed in this period`}>
              <Trans>
                A trade closes when the last share of a purchase is sold. Widen the period if the sales
                happened earlier.
              </Trans>
            </Empty>
          }
        >
          {(data) => <TradesTable rows={data.closed} currency={currency} closed />}
        </Async>
      </Panel>

      <Panel
        title={t`Still open`}
        note={
          trades.data
            ? plural(trades.data.open.length, { one: "# open trade", other: "# open trades" })
            : undefined
        }
        table
      >
        <Async
          query={trades}
          isEmpty={(data) => data.open.length === 0}
          empty={
            <Empty title={t`Nothing is held`}>
              <Trans>Every purchase has been sold out; only closed trades remain.</Trans>
            </Empty>
          }
        >
          {(data) => <TradesTable rows={data.open} currency={currency} closed={false} />}
        </Async>
      </Panel>
    </Page>
  );
}
