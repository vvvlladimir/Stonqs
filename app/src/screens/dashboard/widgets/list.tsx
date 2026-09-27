import { Trans, useLingui } from "@lingui/react/macro";
import { Async, DayMark, List, ListRow, Money, Percent } from "../../../components/ui";
import { useDashboard, useExpectedDividends, usePlanProjection, usePositions } from "../../../lib/queries";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { formatDay } from "../../../lib/format";
import { accountKindLabel } from "../../../lib/kinds";
import { periodOf, type WidgetProps, sourceOf } from "./model";
import { Logo } from "../../../components/domain/Instrument";
import { SecurityLink } from "../../../components/domain/SecurityCardProvider";
import { CrossingLog, EventList } from "../../../components/domain/SecurityAlerts";
import { useAlertCrossings, useSecurityEvents, useWatchlistRows, useWatchlists } from "../../../lib/queries";
import type { CrossingRow } from "../../../lib/types";

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
              lead={<Logo symbol={row.symbol} name={row.name} />}
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
      {(rows) => <CrossingLog rows={family(rows).slice(0, count)} compact />}
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
      {(rows) => <EventList rows={rows.slice(0, count)} named compact />}
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
                lead={<Logo symbol={row.symbol} name={row.name} />}
                title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
                sub={row.name}
                value={<Money value={row.market_value_base} currency={data.base_currency} />}
                meta={
                  <>
                    <Percent value={row.weight} digits={1} dim />
                    {row.day_change && (
                      <>
                        {" · "}
                        <Percent value={row.day_change} signed />
                      </>
                    )}
                  </>
                }
              />
            ))}
        </List>
      )}
    </Async>
  );
}

export function CashWidget({ widget, date }: WidgetProps) {
  const { i18n } = useLingui();
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
        const accountOf = new Map(data.accounts.map((a) => [a.id, a]));
        // An account holds a balance per currency it was ever paid in, so one account can be
        // two rows — a euro deposit that settled a Hong Kong purchase is one. The currency then
        // names the row: two lines reading "Test · Cash" look like two accounts.
        const currencies = new Map<string, number>();
        for (const balance of data.cash)
          currencies.set(balance.account_id, (currencies.get(balance.account_id) ?? 0) + 1);
        return (
          <List>
            {data.cash.map((balance) => {
              const account = accountOf.get(balance.account_id);
              const split = (currencies.get(balance.account_id) ?? 0) > 1;
              return (
                <ListRow
                  key={`${balance.account_id}:${balance.currency}`}
                  title={account?.name ?? balance.account_id}
                  sub={split ? balance.currency : account ? accountKindLabel(i18n, account.kind) : undefined}
                  value={<Money value={balance.amount} currency={balance.currency} digits={0} />}
                />
              );
            })}
          </List>
        );
      }}
    </Async>
  );
}

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
              lead={<DayMark date={c.date} year={false} />}
              title={c.plan_name}
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
              lead={<DayMark date={row.pay_date ?? row.ex_date} year={false} />}
              title={<SecurityLink id={row.security_id}>{row.symbol}</SecurityLink>}
              sub={row.pay_date ? row.name : t`ex-date ${formatDay(row.ex_date)}`}
              value={<Money value={row.net_base ?? row.gross_base} currency={data.base_currency} />}
              meta={row.reported ? t`announced` : t`estimated`}
            />
          ))}
        </List>
      )}
    </Async>
  );
}
