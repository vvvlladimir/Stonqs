import { useLingui } from "@lingui/react/macro";
import {
  Check,
  CheckAll,
  DataTable,
  ListRow,
  Money,
  Quantity,
  useMenu,
  useSelection,
  type MenuItem,
  type Section,
} from "../../components/ui";
import { formatDay, formatMonth, formatMonthName, toNumber } from "../../lib/format";
import { KindTag } from "../../components/domain/KindTag";
import type { TransactionRow, TransactionsData } from "../../lib/types";
import { badgeTone, groupByYear } from "./model";
import { SecurityLink } from "../../components/domain/SecurityCardProvider";

/** The journal itself: rows grouped by year and month, with the core's totals on the headings. */
export function JournalTable({
  data,
  rows,
  currency,
  selection,
  menu,
  itemsFor,
}: {
  data: TransactionsData;
  rows: TransactionRow[];
  currency: string;
  selection: ReturnType<typeof useSelection>;
  menu: ReturnType<typeof useMenu>;
  itemsFor: (row: TransactionRow) => MenuItem[];
}) {
  const { t } = useLingui();
  const sections = useMonthSections(data, rows, currency, selection);

  return (
    <DataTable
      sections={sections}
      rowKey={(row) => row.id}
      rowProps={(row) => menu.row(row.id, itemsFor(row))}
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
              label={t`Select the transaction of ${row.date}`}
            />
          ),
        },
        {
          key: "date",
          sort: (row) => row.date,
          header: t`Date`,
          align: "left",
          // Inside a month group the day alone identifies the row.
          cell: (row) => (
            <span className="num day" title={formatDay(row.date)}>
              {Number(row.date.slice(8, 10))}
            </span>
          ),
        },
        {
          key: "kind",
          sort: (row) => row.kind,
          header: t`Kind`,
          align: "left",
          cell: (row) => <KindTag kind={row.kind} tone={badgeTone(row.net_base)} />,
        },
        {
          key: "security",
          sort: (row) => row.symbol,
          header: t`Instrument`,
          align: "left",
          cell: (row) =>
            row.symbol ? (
              <SecurityLink id={row.security_id}>
                <span className="nm">{row.symbol}</span>
              </SecurityLink>
            ) : (
              <span className="sub">—</span>
            ),
        },
        {
          key: "account",
          sort: (row) => row.account_name,
          header: t`Account`,
          align: "left",
          cell: (row) => row.account_name,
        },
        {
          key: "quantity",
          sort: (row) => toNumber(row.quantity),
          header: t`Qty`,
          cell: (row) => (row.quantity === "0" ? "—" : <Quantity value={row.quantity} />),
        },
        {
          key: "price",
          sort: (row) => toNumber(row.price),
          header: t`Price`,
          cell: (row) =>
            row.price === "0" ? (
              "—"
            ) : (
              <>
                <Money value={row.price} />
                <span className="cur">{row.currency}</span>
              </>
            ),
        },
        {
          key: "net",
          sort: (row) => toNumber(row.net_base),
          header: t`Amount, ${currency}`,
          cell: (row) => <Money value={row.net_base} signed />,
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
          box
          {...menu.row(row.id, itemsFor(row))}
          pick={
            <Check
              checked={selection.has(row.id)}
              onChange={() => selection.toggle(row.id)}
              label={t`Select the transaction of ${row.date}`}
            />
          }
          lead={<KindTag kind={row.kind} tone={badgeTone(row.net_base)} />}
          title={
            row.symbol ? <SecurityLink id={row.security_id}>{row.symbol}</SecurityLink> : (row.note ?? "—")
          }
          sub={`${formatDay(row.date)} · ${row.account_name}`}
          value={<Money value={row.net_base} currency={currency} signed />}
          actions={menu.button(row.id, itemsFor(row))}
        />
      )}
    />
  );
}

/** Every month is its own section, the core's totals on its heading; the year rides on the first. */
function useMonthSections(
  data: TransactionsData,
  rows: TransactionRow[],
  currency: string,
  selection: ReturnType<typeof useSelection>,
): Array<Section<TransactionRow>> {
  const { t } = useLingui();
  return groupByYear(data, rows).flatMap((year) =>
    year.months.map((group, i) => {
      const ids = group.rows.map((row) => row.id);
      const monthLabel = formatMonth(group.year, group.month);
      return {
        key: `${group.year}-${group.month}`,
        rows: group.rows,
        heads: [
          ...(i === 0
            ? [
                {
                  key: `y${year.year}`,
                  className: "grp--year",
                  cells: (
                    <>
                      <td className="pick">
                        <PickAll
                          ids={year.rows.map((row) => row.id)}
                          label={t`Select all transactions of ${year.year}`}
                          selection={selection}
                        />
                      </td>
                      <td colSpan={6} className="num">
                        {year.year}
                      </td>
                      <td className="r num">
                        <Money value={year.net_base} currency={currency} signed />
                      </td>
                      <td />
                    </>
                  ),
                },
              ]
            : []),
          {
            key: `m${group.year}-${group.month}`,
            className: "grp--month",
            cells: (
              <>
                <td className="pick">
                  <PickAll
                    ids={ids}
                    label={t`Select all transactions of ${monthLabel}`}
                    selection={selection}
                  />
                </td>
                <td colSpan={6}>{formatMonthName(group.month)}</td>
                <td className="r num">
                  <Money value={group.net_base} currency={currency} signed />
                </td>
                <td />
              </>
            ),
          },
        ],
        cardHead: (
          <div className="section__head">
            <PickAll ids={ids} label={t`Select all transactions of ${monthLabel}`} selection={selection} />
            <h2>{monthLabel}</h2>
            <span className="spacer" />
            <Money value={group.net_base} currency={currency} signed />
          </div>
        ),
      };
    }),
  );
}

/** Bulk toggle for a whole year or month, driven by how much of it is already picked. */
function PickAll({
  ids,
  label,
  selection,
}: {
  ids: string[];
  label: string;
  selection: ReturnType<typeof useSelection>;
}) {
  const picked = ids.filter((id) => selection.has(id)).length;
  return (
    <CheckAll
      all={picked === ids.length}
      some={picked > 0 && picked < ids.length}
      onChange={() => selection.setMany(ids, picked !== ids.length)}
      label={label}
    />
  );
}
