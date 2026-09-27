import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { Async, Fact, Facts, List, ListRow, Money, Percent, Rate, Stat } from "../../../components/ui";
import { usePortfolio, usePositionReturns, useRebalance, useTrades } from "../../../lib/queries";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { formatDay } from "../../../lib/format";
import type { RiskReport } from "../../../lib/types";
import { periodOf, useWidgetRisk, useWidgetTarget, type WidgetProps, sourceOf } from "./model";
import { Logo } from "../../../components/domain/Instrument";
import { SecurityLink } from "../../../components/domain/SecurityCardProvider";

export function RiskWidget({ widget, date, period }: WidgetProps) {
  const report = useWidgetRisk(widget, date, period);

  if (!report)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return <Async query={report}>{(data) => <RiskRows metrics={data.metrics} />}</Async>;
}

function RiskRows({ metrics }: { metrics: RiskReport["metrics"] }) {
  const { t } = useLingui();
  const days = metrics.longest_drawdown_days;
  return (
    <Facts>
      <Fact label={t`Volatility`} value={<Rate value={metrics.volatility} />} />
      <Fact label={t`Sharpe`} value={<Stat value={metrics.sharpe} />} />
      {/* How far below the peak the period ENDS, which is the one a reader feels; the deepest
          episode below it may have been recovered from years ago. */}
      <Fact label={t`Now under peak`} value={<Rate value={metrics.current_drawdown} tone />} />
      <Fact
        label={t`Deepest`}
        value={metrics.max_drawdown ? <Rate value={metrics.max_drawdown.depth} tone /> : "—"}
      />
      <Fact
        label={t`Longest`}
        value={
          days === null ? (
            "—"
          ) : (
            <span className="num">
              <Plural value={days} one="# day" other="# days" />
            </span>
          )
        }
      />
    </Facts>
  );
}

/** The trades the period closed, biggest result first — the ones worth looking at again. */
export function TradesWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const trades = useTrades(range, "POSITION", source);
  const count = Number(widget.cfg.count) || 6;

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <Async
      query={trades}
      isEmpty={(data) => data.closed.length === 0}
      empty={
        <p className="muted">
          <Trans>Nothing was closed in this period.</Trans>
        </p>
      }
    >
      {(data) => (
        <List>
          {/* Sort by size of result regardless of sign: the worst trade is as instructive. */}
          {[...data.closed]
            .sort((a, b) => Math.abs(Number(b.pnl_base)) - Math.abs(Number(a.pnl_base)))
            .slice(0, count)
            .map((row, index) => (
              <ListRow
                key={`${row.security_id}:${row.opened_at}:${index}`}
                lead={<Logo symbol={row.symbol} name={row.name} />}
                title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
                sub={row.closed_at ? formatDay(row.closed_at) : undefined}
                value={<Money value={row.pnl_base} currency={data.base_currency} signed />}
                meta={row.return_pct ? <Percent value={row.return_pct} signed /> : "—"}
              />
            ))}
        </List>
      )}
    </Async>
  );
}

/** What each category should be worth against what it is worth — money, not percentages:
 * a drift of two points means nothing until it is the price of a trade. */
export function TargetValueWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const target = useWidgetTarget(widget);
  const portfolio = usePortfolio();
  const plan = useRebalance(target.id, date, null, true, source);
  const count = Number(widget.cfg.count) || 6;
  const currency = portfolio.data?.base_currency ?? "";

  if (target.id === null)
    return (
      <p className="muted">
        <Trans>No target has been set up yet.</Trans>
      </p>
    );

  return (
    <Async
      query={plan}
      isEmpty={(data) => data.items.length === 0}
      empty={
        <p className="muted">
          <Trans>The target has no nodes.</Trans>
        </p>
      }
    >
      {(data) => (
        <List>
          {/* Largest gap first: the row that would move the most money. */}
          {[...data.items]
            .sort((a, b) => Math.abs(Number(b.drift_base)) - Math.abs(Number(a.drift_base)))
            .slice(0, count)
            .map((item) => (
              <ListRow
                key={item.node_id}
                title={item.label}
                sub={<Percent value={item.target_weight} digits={1} />}
                value={<Money value={item.target_base} currency={currency} />}
                meta={<Money value={item.drift_base} currency={currency} signed />}
              />
            ))}
        </List>
      )}
    </Async>
  );
}

/** The best return of the period, whatever the position weighs: a small holding can be the year's
 * best decision, and the contribution waterfall — which ranks by money — would never show it. */
export function PerformersWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const rows = usePositionReturns(range, source);
  const portfolio = usePortfolio();
  const currency = portfolio.data?.base_currency ?? "";
  const count = Number(widget.cfg.count) || 6;

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <Async
      query={rows}
      isEmpty={(data) => data.every((row) => row.twr === null)}
      empty={
        <p className="muted">
          <Trans>No instrument has a return over this period.</Trans>
        </p>
      }
    >
      {(data) => (
        <List>
          {/* A position without a solvable TWR is left out rather than ranked as zero. */}
          {data
            .filter((row): row is typeof row & { twr: string } => row.twr !== null)
            .sort((a, b) => Number(b.twr) - Number(a.twr))
            .slice(0, count)
            .map((row) => (
              <ListRow
                key={row.security_id}
                lead={<Logo symbol={row.symbol} name={row.name} />}
                title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
                sub={row.name}
                value={<Percent value={row.twr} signed />}
                meta={<Money value={row.pnl_base} currency={currency} signed />}
              />
            ))}
        </List>
      )}
    </Async>
  );
}
