import { Trans, useLingui } from "@lingui/react/macro";
import { Async, List, ListRow, Money, Percent, Rate, Stat } from "../../../components/ui";
import {
  useDashboard,
  useExpectedDividends,
  usePlanProjection,
  usePortfolio,
  usePositionReturns,
  usePositions,
  useRebalance,
  useTrades,
} from "../../../lib/queries";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { formatDay } from "../../../lib/format";
import type { RiskReport } from "../../../lib/types";
import { periodOf, useWidgetRisk, useWidgetTarget, type WidgetProps, sourceOf } from "./model";
import { SecurityLink } from "../../../components/domain/SecurityCardProvider";
import { CrossingLog, EventList } from "../../../components/domain/SecurityAlerts";
import { useAlertCrossings, useSecurityEvents, useWatchlistRows, useWatchlists } from "../../../lib/queries";
import type { CrossingRow } from "../../../lib/types";

/**
 * One watchlist, in its own order: the last price and the period's move. Not scoped — a price
 * does not depend on the picker, so the widget offers no data source.
 */
export function WatchlistWidget({ widget, date, period }: WidgetProps) {
  const lists = useWatchlists();
  const chosen = typeof widget.cfg.watchlist === "string" ? widget.cfg.watchlist : "";
  const list = lists.data?.find((l) => l.id === chosen) ?? lists.data?.[0] ?? null;
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const rows = useWatchlistRows(list?.id ?? null, range);
  const count = Number(widget.cfg.count) || 6;

  if (lists.data && !list)
    return (
      <p className="muted">
        <Trans>No watchlist yet.</Trans>
      </p>
    );
  // A disabled query stays pending forever, so a missing period is answered here.
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <Async
      query={rows}
      isEmpty={(data) => data.length === 0}
      empty={
        <p className="muted">
          <Trans>The list is empty.</Trans>
        </p>
      }
    >
      {(data) => (
        <List>
          {data.slice(0, count).map((row) => (
            <ListRow
              key={row.security_id}
              title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
              sub={row.name}
              value={row.price && row.currency ? <Money value={row.price} currency={row.currency} /> : "—"}
              meta={row.period_return ? <Percent value={row.period_return} signed /> : "—"}
            />
          ))}
        </List>
      )}
    </Async>
  );
}

export function LimitsWidget({ widget }: WidgetProps) {
  return <Crossings count={Number(widget.cfg.count) || 6} dates={false} />;
}

export function DatesWidget({ widget }: WidgetProps) {
  return <Crossings count={Number(widget.cfg.count) || 6} dates />;
}

/** The latest log lines of one family: level crossings, or dates that arrived. */
function Crossings({ count, dates }: { count: number; dates: boolean }) {
  const log = useAlertCrossings(100);
  const family = (rows: CrossingRow[]) => rows.filter((row) => (row.kind === "DATE_REACHED") === dates);

  return (
    <Async
      query={log}
      isEmpty={(rows) => family(rows).length === 0}
      empty={
        <p className="muted">
          {dates ? <Trans>No review date has arrived.</Trans> : <Trans>No level has been crossed.</Trans>}
        </p>
      }
    >
      {(rows) => <CrossingLog rows={family(rows).slice(0, count)} />}
    </Async>
  );
}

export function EventsWidget({ widget }: WidgetProps) {
  const count = Number(widget.cfg.count) || 6;
  const events = useSecurityEvents();
  return (
    <Async
      query={events}
      empty={
        <p className="muted">
          <Trans>No notes, dividends or splits yet.</Trans>
        </p>
      }
    >
      {(rows) => <EventList rows={rows.slice(0, count)} named />}
    </Async>
  );
}

export function PositionsWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const positions = usePositions(date, source);
  const count = Number(widget.cfg.count) || 6;

  return (
    <Async
      query={positions}
      isEmpty={(data) => data.rows.length === 0}
      empty={
        <p className="muted">
          <Trans>No open positions.</Trans>
        </p>
      }
    >
      {(data) => (
        <List>
          {/* Sort by numeric value; the formatted strings are for display only. */}
          {[...data.rows]
            .sort((a, b) => Number(b.market_value_base) - Number(a.market_value_base))
            .slice(0, count)
            .map((row) => (
              <ListRow
                key={row.security_id}
                title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
                sub={<Percent value={row.weight} digits={1} />}
                value={<Money value={row.market_value_base} currency={data.base_currency} />}
                meta={row.day_change ? <Percent value={row.day_change} signed /> : "—"}
              />
            ))}
        </List>
      )}
    </Async>
  );
}

export function CashWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const summary = useDashboard(date, source);
  return (
    <Async
      query={summary}
      isEmpty={(data) => data.cash.length === 0}
      empty={
        <p className="muted">
          <Trans>No cash balances.</Trans>
        </p>
      }
    >
      {(data) => {
        const nameOf = new Map(data.accounts.map((a) => [a.id, a.name]));
        return (
          <List>
            {data.cash.map((balance) => (
              <ListRow
                key={`${balance.account_id}:${balance.currency}`}
                title={nameOf.get(balance.account_id) ?? balance.account_id}
                value={<Money value={balance.amount} currency={balance.currency} />}
              />
            ))}
          </List>
        );
      }}
    </Async>
  );
}

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
  return (
    <List>
      <ListRow title={t`Volatility`} value={<Rate value={metrics.volatility} />} />
      <ListRow title={t`Sharpe`} value={<Stat value={metrics.sharpe} />} />
      <ListRow
        title={t`Maximum drawdown`}
        value={metrics.max_drawdown ? <Rate value={metrics.max_drawdown.depth} className="neg" /> : "—"}
      />
      {metrics.max_drawdown && (
        <div className="w__foot">
          <Trans>trough {formatDay(metrics.max_drawdown.trough)}</Trans> ·{" "}
          {metrics.max_drawdown.recovered
            ? t`recovered ${formatDay(metrics.max_drawdown.recovered)}`
            : t`not recovered`}
        </div>
      )}
    </List>
  );
}

/** The trades the period closed, biggest result first — the ones worth looking at again. */
export function TradesWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const trades = useTrades(range, source);
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

/**
 * The contributions still to come, soonest first. Not scoped: a plan is an intention about the
 * portfolio, and narrowing the picker must not hide next month's savings.
 */
export function ContributionsWidget({ widget }: WidgetProps) {
  const count = Number(widget.cfg.count) || 6;
  // A window wide enough that a yearly plan still shows up in the list.
  const projection = usePlanProjection(12);

  return (
    <Async
      query={projection}
      isEmpty={(data) => data.contributions.length === 0}
      empty={
        <p className="muted">
          <Trans>No plans yet.</Trans>
        </p>
      }
    >
      {(data) => (
        <List>
          {data.contributions.slice(0, count).map((c, index) => (
            <ListRow
              key={`${c.plan_id}:${c.date}:${index}`}
              title={c.plan_name}
              sub={formatDay(c.date)}
              value={<Money value={c.amount} currency={c.currency} />}
              meta={
                c.currency === data.base_currency ? undefined : (
                  <Money value={c.amount_base} currency={data.base_currency} />
                )
              }
            />
          ))}
        </List>
      )}
    </Async>
  );
}

/**
 * Dividends the open positions should pay next, soonest first: the source's reported payments
 * carried a year forward. The amount is net once a payment was received to learn the
 * withholding from, gross before that.
 */
export function ExpectedDividendsWidget({ widget }: WidgetProps) {
  const { t } = useLingui();
  const source = sourceOf(widget);
  const count = Number(widget.cfg.count) || 6;
  // A year, so that an annual payer shows up at all.
  const expected = useExpectedDividends(12, source);

  return (
    <Async
      query={expected}
      isEmpty={(data) => data.rows.length === 0}
      empty={
        <p className="muted">
          <Trans>No dividends expected.</Trans>
        </p>
      }
    >
      {(data) => (
        <List>
          {data.rows.slice(0, count).map((row) => (
            <ListRow
              key={`${row.security_id}:${row.ex_date}`}
              title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
              sub={row.pay_date ? formatDay(row.pay_date) : t`ex-date ${formatDay(row.ex_date)}`}
              value={<Money value={row.net_base ?? row.gross_base} currency={data.base_currency} />}
              meta={row.reported ? t`announced` : t`estimated`}
            />
          ))}
        </List>
      )}
    </Async>
  );
}
