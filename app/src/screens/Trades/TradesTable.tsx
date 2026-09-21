import { useLingui } from "@lingui/react/macro";
import { DataTable, DayMark, ListRow, Money, Percent, Quantity } from "../../components/ui";
import { Instrument } from "../../components/domain/Instrument";
import { formatDay, toNumber } from "../../lib/format";
import type { TradeRow } from "../../lib/types";
import { days } from "./model";

/** Open and closed trades differ by one column, so one table renders both. */
export function TradesTable({
  rows,
  currency,
  closed,
}: {
  rows: TradeRow[];
  currency: string;
  /** A closed trade has a second date and an exit; an open one only repeats its opening. */
  closed: boolean;
}) {
  const { t } = useLingui();
  const instrument = (row: TradeRow) => (
    <Instrument id={row.security_id} symbol={row.symbol} name={row.name} />
  );

  return (
    <DataTable
      rows={rows.map((row, index) => ({ ...row, index }))}
      rowKey={(row) => String(row.index)}
      columns={[
        {
          key: "security",
          sort: (row) => row.name || row.symbol,
          header: t`Instrument`,
          align: "left",
          cell: instrument,
        },
        {
          key: "opened",
          sort: (row) => row.opened_at,
          header: t`Opened`,
          align: "left",
          className: "nm",
          width: "3.5rem",
          only: "wide",
          cell: (row) => <DayMark date={row.opened_at} />,
        },
        // An open trade has no closing date, and repeating its opening under a second header
        // only reads as two different facts.
        ...(closed
          ? [
              {
                key: "closed",
                header: t`Closed`,
                align: "left" as const,
                className: "nm",
                width: "3.5rem",
                only: "wide" as const,
                sort: (row: TradeRow) => row.closed_at,
                cell: (row: TradeRow) => (row.closed_at ? <DayMark date={row.closed_at} /> : "—"),
              },
            ]
          : []),
        {
          key: "held",
          sort: (row) => row.holding_days,
          header: t`Held`,
          only: "wide",
          cell: (row) => days(row.holding_days),
        },
        {
          key: "qty",
          header: t`Qty`,
          sort: (row) => toNumber(row.quantity),
          cell: (row) => <Quantity value={row.quantity} />,
        },
        {
          key: "entry",
          sort: (row) => toNumber(row.entry_value_base),
          header: t`Entry`,
          only: "wide",
          cell: (row) => <Money value={row.entry_value_base} />,
        },
        {
          key: "exit",
          sort: (row) => toNumber(row.exit_value_base),
          header: closed ? t`Exit` : t`Value`,
          cell: (row) => <Money value={row.exit_value_base} />,
        },
        {
          key: "return",
          sort: (row) => row.return_pct,
          header: t`Return`,
          factLabel: t`return`,
          cell: (row) =>
            row.return_pct === null ? "—" : <Percent value={row.return_pct} digits={1} signed />,
        },
        {
          key: "irr",
          sort: (row) => row.irr,
          header: t`IRR`,
          only: "wide",
          cell: (row) => (row.irr === null ? "—" : <Percent value={row.irr} digits={1} signed />),
        },
        {
          key: "pnl",
          sort: (row) => toNumber(row.pnl_base),
          header: t`Result, ${currency}`,
          card: "value",
          cell: (row) => <Money value={row.pnl_base} signed />,
        },
      ]}
      card={(row) => (
        <ListRow
          as="li"
          box
          top
          title={instrument(row)}
          value={<Money value={row.pnl_base} currency={currency} signed />}
          meta={row.return_pct === null ? undefined : <Percent value={row.return_pct} digits={1} signed />}
          foot={
            <>
              <span>
                <Quantity value={row.quantity} /> · {days(row.holding_days)}
              </span>
              <span>{row.closed_at ? formatDay(row.closed_at) : formatDay(row.opened_at)}</span>
            </>
          }
        />
      )}
    />
  );
}
