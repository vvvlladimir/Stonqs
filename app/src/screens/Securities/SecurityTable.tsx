import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { Instrument, Logo } from "../../components/domain/Instrument";
import {
  Check,
  CheckAll,
  DataTable,
  Empty,
  ListRow,
  Money,
  Num,
  Tag,
  useMenu,
  useSelection,
  type MenuItem,
} from "../../components/ui";
import { toNumber } from "../../lib/format";
import type { SecurityRow } from "../../lib/types";
import { SecurityLink } from "../../components/domain/SecurityCardProvider";

export function SecurityTable({
  rows,
  selection,
  menu,
  itemsFor,
}: {
  rows: SecurityRow[];
  selection: ReturnType<typeof useSelection>;
  menu: ReturnType<typeof useMenu>;
  itemsFor: (row: SecurityRow) => MenuItem[];
}) {
  const { t } = useLingui();
  return (
    <DataTable
      rows={rows}
      rowKey={(row) => row.id}
      rowProps={(row) => menu.row(row.id, itemsFor(row))}
      cards="lines"
      empty={
        <Empty title={t`Nothing found`}>
          <Trans>
            Either the directory is empty or the search hid everything. Instruments are added with "Add
            instrument", or appear on their own when a broker export is imported.
          </Trans>
        </Empty>
      }
      columns={[
        {
          key: "pick",
          header: <CheckAll all={selection.all} some={selection.some} onChange={selection.toggleAll} />,
          align: "left",
          className: "pick",
          cell: (row) => (
            <Check
              checked={selection.has(row.id)}
              onChange={() => selection.toggle(row.id)}
              label={t`Select ${row.symbol}`}
            />
          ),
        },
        {
          key: "instrument",
          sort: (row) => row.name || row.symbol,
          header: t`Instrument`,
          align: "left",
          cell: (row) => <Instrument id={row.id} symbol={row.symbol} name={row.name} kind={row.kind} />,
        },
        {
          key: "isin",
          sort: (row) => row.isin,
          header: "ISIN",
          align: "left",
          className: "num sub",
          cell: (row) => (row.isin ? <SecurityLink id={row.id}>{row.isin}</SecurityLink> : "—"),
        },
        {
          key: "source",
          sort: (row) => row.data_source,
          header: t`Source`,
          align: "left",
          cell: (row) => (row.data_source ? <Tag>{row.data_source}</Tag> : <Tag warn>{t`manual`}</Tag>),
        },
        { key: "price", header: t`Last price`, sort: (row) => toNumber(row.last_close), cell: priceOf },
        {
          key: "history",
          header: t`Quote history`,
          className: "sub",
          ariaLabel: t`Quote history: the period cached locally and how many quotes it holds. Red means the series is far shorter than the instrument has been held — usually the wrong venue`,
          sort: (row) => row.quote_count,
          cell: HistoryCell,
        },
        {
          key: "acts",
          align: "left",
          className: "acts",
          cell: (row) => menu.button(row.id, itemsFor(row)),
        },
      ]}
      card={(row) => (
        <ListRow
          as="li"
          {...menu.row(row.id, itemsFor(row))}
          pick={
            <Check
              checked={selection.has(row.id)}
              onChange={() => selection.toggle(row.id)}
              label={t`Select ${row.symbol}`}
            />
          }
          lead={<Logo symbol={row.symbol} name={row.name} />}
          title={<SecurityLink id={row.id}>{row.name || row.symbol}</SecurityLink>}
          sub={
            <>
              <Num>
                <SecurityLink id={row.id}>{row.isin ?? row.symbol}</SecurityLink>
              </Num>
              <ListingCell row={row} />
            </>
          }
          value={priceOf(row)}
          actions={menu.button(row.id, itemsFor(row))}
        />
      )}
    />
  );
}

/** Show the selected listing and its actual quote currency on narrow screens. */
function ListingCell({ row }: { row: SecurityRow }) {
  const { t } = useLingui();
  if (!row.mic) return <Tag warn>{t`not chosen`}</Tag>;
  return (
    <span className="inline">
      <Tag>{row.mic}</Tag>
      <span className="sub">
        {[row.venue, row.quote_currency ?? row.currency].filter(Boolean).join(" · ")}
      </span>
    </span>
  );
}

/** Last price is always paired with its quote currency. */
function priceOf(row: SecurityRow) {
  if (row.last_close === null) return <span className="dim">—</span>;
  return (
    <>
      <Money value={row.last_close} />
      <span className="cur">{row.quote_currency ?? row.currency}</span>
    </>
  );
}

/**
 * The cached quote period and count. A series far shorter than the instrument has been held is
 * the symptom of a ticker on a venue this source does not quote, so it is stated in red instead
 * of read as a history: the refresh moves such an instrument by itself, and "Venues…" does the
 * rest where it cannot.
 */
function HistoryCell(row: SecurityRow) {
  const thin = row.sparse_history;
  if (row.quote_count === 0 || !row.coverage_from || !row.coverage_to)
    return (
      <span className={thin ? "neg" : "dim"}>
        <Trans>none</Trans>
      </span>
    );
  return (
    <>
      <div>
        {row.coverage_from.slice(0, 7)} — {row.coverage_to.slice(0, 7)}
      </div>
      <div className={thin ? "neg" : "dim"}>
        <Plural value={row.quote_count} one="# quote" other="# quotes" />
      </div>
    </>
  );
}
