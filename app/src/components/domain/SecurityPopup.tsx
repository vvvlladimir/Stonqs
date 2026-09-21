import { useState } from "react";
import { today } from "../../lib/api";
import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import {
  QUOTE_WINDOW_DAYS,
  useAlerts,
  usePositionReturn,
  usePositions,
  useQuoteWindow,
  useSecurities,
  useSecurityEvents,
  useTransactions,
} from "../../lib/queries";
import { alertToInput, newAlert, newNote, noteToInput } from "../../lib/alerts";
import { AlertDialog, NoteDialog } from "./AlertDialogs";
import { AlertList, EventList } from "./SecurityAlerts";
import { WatchlistPicker } from "./WatchlistPicker";
import { useNav } from "../../lib/nav";
import { AssignDialog } from "./AssignDialog";
import { ClassificationPanel } from "./SecurityClassification";
import { Logo } from "./Instrument";
import { Area, Chart, Grid, Marker, indexScale, valueAxis } from "../charts";
import {
  Async,
  Buttons,
  DataTable,
  Metric,
  Modal,
  Money,
  Num,
  Panel,
  Percent,
  Quantity,
  Rate,
  Skeleton,
  SkeletonRows,
  Tag,
} from "../ui";
import { CH, path, toPlotNumber } from "../../lib/plot";
import { formatDay, formatDecimal, formatQuantity, signOf, toNumber, toneClass } from "../../lib/format";
import { securityKindLabel } from "../../lib/kinds";
import { KindTag } from "./KindTag";
import type {
  AlertInput,
  PositionRow,
  Quote,
  SecurityEventInput,
  SecurityRow,
  TaxonomyData,
  TransactionRow,
} from "../../lib/types";

/**
 * The instrument's card: market history, facts, classification and its latest transactions.
 *
 * It is opened from an id alone, because a ticker is printed on every screen and most of them
 * hold nothing else. An instrument that is sold out has no position row, and the card then shows
 * what is still true about it rather than refusing to open.
 */

/** Number of recent transactions shown before opening the full journal. */
const RECENT = 6;

/** Facts a held position ends up printing; the placeholder reserves exactly that many lines. */
const FACTS_WHILE_LOADING = 12;

export function SecurityCard({ securityId, onClose }: { securityId: string; onClose: () => void }) {
  const { t, i18n } = useLingui();
  const nav = useNav();
  const to = today();

  const positions = usePositions(to);
  const securities = useSecurities();
  const row: PositionRow | undefined = positions.data?.rows.find((r) => r.security_id === securityId);
  const security: SecurityRow | undefined = securities.data?.find((s) => s.id === securityId);
  const currency = positions.data?.base_currency ?? "";
  const symbol = row?.symbol ?? security?.symbol ?? securityId;
  const name = row?.name ?? security?.name ?? "";

  const quotes = useQuoteWindow(securityId);
  // Return since the first lot; without a position there is no window to measure over.
  const returns = usePositionReturn(securityId, row?.lots[0]?.acquired_at ?? null, to);
  const transactions = useTransactions({ security_id: securityId });
  // The split dialog opens over this one; while it stands, Escape belongs to it.
  const [splitting, setSplitting] = useState<TaxonomyData | null>(null);
  const alerts = useAlerts(securityId);
  const events = useSecurityEvents(securityId);
  // Editors over this card; like the split dialog, each takes Escape while it stands.
  const [alertDraft, setAlertDraft] = useState<AlertInput | null>(null);
  const [note, setNote] = useState<{ draft: SecurityEventInput; reported: boolean } | null>(null);
  const [watching, setWatching] = useState(false);

  const rows = transactions.data?.rows ?? [];
  // Two queries decide the shape of the card — whether it is held, and what it is. Until both
  // have answered, every block stands in its final size rather than growing into it.
  const loading = positions.isPending || securities.isPending;

  return (
    <Modal
      title={name || symbol}
      onClose={() => {
        if (splitting) setSplitting(null);
        else if (alertDraft) setAlertDraft(null);
        else if (note) setNote(null);
        else if (watching) setWatching(false);
        else onClose();
      }}
      wide
      foot={
        <>
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            <Trans>Close</Trans>
          </button>
          <button type="button" className="btn btn--ghost" onClick={() => setWatching(true)}>
            <Trans>Watchlists…</Trans>
          </button>
          <button
            type="button"
            className="btn btn--ghost"
            onClick={() => {
              onClose();
              nav.go("transactions", symbol);
            }}
          >
            <Trans>Show transactions</Trans>
          </button>
          {(loading || security) && (
            <button
              type="button"
              className="btn"
              disabled={loading}
              onClick={() => {
                onClose();
                nav.go("securities", symbol);
              }}
            >
              <Trans>Open in the directory</Trans>
            </button>
          )}
        </>
      }
    >
      {watching && <WatchlistPicker securityId={securityId} onClose={() => setWatching(false)} />}
      <div className="secpop__top">
        <Logo symbol={symbol} name={name} />
        <div className="min0">
          <div className="secpop__t">{symbol}</div>
          <div className="row__sub">
            {loading ? (
              <Skeleton w="12rem" h="0.9rem" />
            ) : (
              <>
                {security?.isin && <Num>{security.isin}</Num>}
                <span>{security?.venue ?? t`no venue chosen`}</span>
                {security && <Tag>{securityKindLabel(i18n, security.kind)}</Tag>}
              </>
            )}
          </div>
        </div>
        <span className="spacer" />
        {loading && (
          <div className="secpop__v">
            <Skeleton w="7rem" h="1.6rem" />
            <div>
              <Skeleton w="4rem" h="0.9rem" />
            </div>
          </div>
        )}
        {!loading && row && (
          <div className="secpop__v">
            <Money value={row.market_value_base} currency={currency} />
            {row.day_change !== null && (
              <div>
                <Trans>
                  <Percent value={row.day_change} signed /> today
                </Trans>
              </div>
            )}
          </div>
        )}
      </div>

      {loading ? (
        <div className="secpop__grid">
          {[t`Share of the source`, t`Return`, t`Unrealized`, t`Dividends`].map((label) => (
            <Metric key={label} label={label} value={<Skeleton w="4.5rem" h="1.3rem" />} />
          ))}
        </div>
      ) : row ? (
        <div className="secpop__grid">
          <Metric label={t`Share of the source`} value={<Percent value={row.weight} digits={1} />} />
          <Metric
            label={t`Return`}
            value={
              returns.isPending ? (
                <Skeleton w="4.5rem" h="1.3rem" />
              ) : returns.data?.twr ? (
                <Percent value={returns.data.twr} signed digits={1} tone={false} />
              ) : (
                "—"
              )
            }
            hint={t`TWR since the first purchase`}
            tone={returns.data?.twr ? signOf(returns.data.twr) : "neutral"}
          />
          <Metric
            label={t`Unrealized`}
            value={<Money value={row.unrealized_pnl_base} signed tone={false} />}
            tone={signOf(row.unrealized_pnl_base)}
          />
          <Metric label={t`Dividends`} value={<Money value={row.dividends_base} />} hint={t`all time`} />
        </div>
      ) : (
        <p className="panel__note secpop__none">
          <Trans>Nothing is held in this instrument now — what follows is its own history.</Trans>
        </p>
      )}

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

      <div className="grid-2 stack">
        <Panel title={t`Facts`}>
          {loading && <SkeletonRows rows={FACTS_WHILE_LOADING} />}
          {!loading && (
            <dl className="facts">
              {row && (
                <>
                  <Fact k={t`Quantity`} v={<Quantity value={row.quantity} />} />
                  <Fact
                    k={t`Price`}
                    v={
                      <>
                        <Money value={row.price} /> {row.currency}
                      </>
                    }
                  />
                  <Fact k={t`Cost basis`} v={<Money value={row.cost_basis_base} currency={currency} />} />
                  <Fact
                    k={t`Unrealized`}
                    v={<Money value={row.unrealized_pnl_base} signed tone={false} />}
                    tone={toneClass(row.unrealized_pnl_base)}
                  />
                  <Fact
                    k={t`Realized`}
                    v={
                      signOf(row.realized_pnl_base) === "zero" ? (
                        t`nothing sold`
                      ) : (
                        <Money value={row.realized_pnl_base} signed tone={false} />
                      )
                    }
                    tone={toneClass(row.realized_pnl_base)}
                  />
                  <Fact
                    k={t`Dividends`}
                    v={
                      signOf(row.dividends_base) === "zero" ? (
                        t`pays none`
                      ) : (
                        <Money value={row.dividends_base} currency={currency} />
                      )
                    }
                  />
                  <Fact k="IRR" v={returns.data?.xirr ? <Percent value={returns.data.xirr} signed /> : "—"} />
                </>
              )}
              <Fact k="ISIN" v={security?.isin ?? "—"} />
              {security?.wkn && <Fact k="WKN" v={security.wkn} />}
              <Fact k={t`Venue`} v={listingOf(i18n, security)} />
              <Fact k={t`Quote source`} v={security?.data_source ?? <Tag warn>{t`manual`}</Tag>} />
              <Fact k={t`Quote history`} v={historyOf(i18n, security)} />
              <Fact k={t`Quantity step`} v={stepOf(i18n, security)} />
              {row && <Fact k={t`Held in`} v={row.accounts.map((a) => a.account_name).join(", ") || "—"} />}
            </dl>
          )}
        </Panel>

        <Panel title={t`Classification`}>
          <ClassificationPanel securityId={securityId} security={security} onSplit={setSplitting} />
        </Panel>
      </div>

      <Panel
        title={t`Alerts and events`}
        tools={
          <Buttons>
            <button
              type="button"
              className="btn btn--sm btn--ghost"
              onClick={() => setNote({ draft: newNote(securityId), reported: false })}
            >
              <Trans>Add note</Trans>
            </button>
            <button
              type="button"
              className="btn btn--sm"
              disabled={!security}
              onClick={() => security && setAlertDraft(newAlert(security))}
            >
              <Trans>New alert</Trans>
            </button>
          </Buttons>
        }
      >
        {alerts.data && alerts.data.length > 0 && (
          <AlertList rows={alerts.data} onEdit={(r) => setAlertDraft(alertToInput(r.alert))} />
        )}
        <Async
          query={events}
          pending={<SkeletonRows rows={2} />}
          empty={
            <p className="muted">
              <Trans>No notes, dividends or splits for this instrument.</Trans>
            </p>
          }
        >
          {(list) => (
            <EventList
              rows={list.slice(0, RECENT)}
              onEditNote={(e) => setNote({ draft: noteToInput(e), reported: e.kind !== "NOTE" })}
            />
          )}
        </Async>
      </Panel>

      {alertDraft && (
        <AlertDialog
          draft={alertDraft}
          securities={security ? [security] : []}
          fixed
          onChange={setAlertDraft}
          onClose={() => setAlertDraft(null)}
        />
      )}
      {note && (
        <NoteDialog
          draft={note.draft}
          reported={note.reported}
          onChange={(next) => setNote({ ...note, draft: next })}
          onClose={() => setNote(null)}
        />
      )}

      {splitting && (
        <AssignDialog
          taxonomy={splitting}
          subjectId={securityId}
          name={name ? `${symbol} · ${name}` : symbol}
          onClose={() => setSplitting(null)}
          onSaved={() => setSplitting(null)}
        />
      )}

      {transactions.isPending && (
        <Panel title={t`Recent transactions`}>
          <SkeletonRows rows={RECENT} />
        </Panel>
      )}

      {rows.length > 0 && (
        <Panel
          title={t`Recent transactions`}
          tools={
            <span className="panel__note">
              <Trans>{rows.length} in total</Trans>
            </span>
          }
          table
        >
          <DataTable
            card={false}
            rows={rows.slice(0, RECENT)}
            rowKey={(t: TransactionRow) => t.id}
            columns={[
              {
                key: "date",
                sort: (t: TransactionRow) => t.date,
                header: t`Date`,
                align: "left",
                className: "num",
                cell: (t: TransactionRow) => formatDay(t.date),
              },
              {
                key: "kind",
                sort: (t: TransactionRow) => t.kind,
                header: t`Kind`,
                align: "left",
                cell: (t: TransactionRow) => (
                  <KindTag kind={t.kind} tone={signOf(t.net_base) === "negative" ? "out" : "in"} />
                ),
              },
              {
                key: "quantity",
                sort: (t: TransactionRow) => toNumber(t.quantity),
                header: t`Qty`,
                cell: (t: TransactionRow) => (t.quantity === "0" ? "" : <Quantity value={t.quantity} />),
              },
              {
                key: "price",
                sort: (t: TransactionRow) => toNumber(t.price),
                header: t`Price`,
                cell: (t: TransactionRow) => (t.price === "0" ? "" : <Money value={t.price} />),
              },
              {
                key: "amount",
                sort: (t: TransactionRow) => toNumber(t.amount),
                header: t`Amount`,
                cell: (t: TransactionRow) => <Money value={t.amount} currency={t.currency} />,
              },
            ]}
          />
        </Panel>
      )}
    </Modal>
  );
}

function Fact({ k, v, tone }: { k: string; v: React.ReactNode; tone?: string }) {
  return (
    <>
      <dt>{k}</dt>
      <dd className={tone}>{v}</dd>
    </>
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

function listingOf(i18n: I18n, security?: SecurityRow): React.ReactNode {
  if (!security) return "—";
  if (!security.mic) return <Tag warn>{i18n._(msg`not chosen`)}</Tag>;
  return [security.venue, security.mic, security.quote_currency ?? security.currency]
    .filter(Boolean)
    .join(" · ");
}

function historyOf(i18n: I18n, security?: SecurityRow): string {
  if (!security || security.quote_count === 0 || !security.coverage_from || !security.coverage_to) {
    return i18n._(msg`no quotes`);
  }
  const count = formatDecimal(String(security.quote_count), { digits: 0 });
  return `${security.coverage_from} — ${security.coverage_to} · ${count}`;
}

/** Shows the effective quantity step and its source. */
function stepOf(i18n: I18n, security?: SecurityRow): string {
  if (!security) return "—";
  const effective = formatQuantity(security.effective_quantity_step);
  if (security.quantity_step !== null) return effective;
  return security.quantity_step_observed !== null
    ? i18n._(msg`${effective} (from trades)`)
    : i18n._(msg`${effective} (from the kind)`);
}
