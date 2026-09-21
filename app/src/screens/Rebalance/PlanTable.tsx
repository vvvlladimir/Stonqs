import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { Logo } from "../../components/domain/Instrument";
import { Badge, Bar, DataTable, Money, Percent, Quantity, Swatch } from "../../components/ui";
import { toNumber } from "../../lib/format";
import { slotOfNode } from "../../lib/taxonomy";
import type { RebalanceItem, RebalanceTrade, SecurityRow, TaxonomyData } from "../../lib/types";
import { nameOf, type CashRow, type PlanRow } from "./model";
import { SecurityLink } from "../../components/domain/SecurityCardProvider";

/** Trades and cash deposits in one table: both are "what to do to hit the target". */
export function PlanTable({
  trades,
  cashRows,
  currency,
  taxonomy,
  securities,
}: {
  trades: Array<{ trade: RebalanceTrade; item: RebalanceItem }>;
  cashRows: CashRow[];
  currency: string;
  taxonomy: TaxonomyData | undefined;
  securities: SecurityRow[] | undefined;
}) {
  const { t } = useLingui();
  const tradeRows: PlanRow[] = trades.map(({ trade, item }) => ({
    kind: "trade",
    key: `${item.node_id}-${trade.security_id}`,
    trade,
    item,
  }));
  const cashPlanRows: PlanRow[] = cashRows.map((cash) => ({ kind: "cash", key: cash.key, cash }));

  return (
    <DataTable
      variant="rows"
      card={false}
      rowKey={(row) => row.key}
      sections={[
        { key: "trades", rows: tradeRows },
        ...(cashPlanRows.length > 0
          ? [
              {
                key: "cash",
                rows: cashPlanRows,
                heads: [
                  {
                    key: "cash-head",
                    cells: (
                      <td colSpan={6}>
                        <Trans>Cash is part of the plan — it is paid in, not bought</Trans>
                      </td>
                    ),
                  },
                ],
              },
            ]
          : []),
      ]}
      columns={[
        {
          key: "subject",
          sort: (row) => (row.kind === "trade" ? row.trade.symbol : row.cash.account_name),
          header: t`Instrument`,
          align: "left",
          cell: (row) => <SubjectCell row={row} securities={securities} />,
        },
        {
          key: "node",
          sort: (row) => (row.kind === "trade" ? row.item.label : row.cash.node),
          header: t`Category`,
          align: "left",
          className: "table-wide",
          cell: (row) => nodeCell(row, taxonomy),
        },
        {
          key: "price",
          sort: (row) => (row.kind === "trade" ? toNumber(row.trade.price_base) : null),
          header: t`Price`,
          className: "money",
          cell: (row) =>
            row.kind === "trade" ? (
              <>
                <Money value={row.trade.price_base} digits={2} /> <span className="dim">{currency}</span>
              </>
            ) : (
              <span className="dim">—</span>
            ),
        },
        {
          key: "quantity",
          sort: (row) => (row.kind === "trade" ? Math.abs(Number(row.trade.quantity)) : null),
          header: t`Quantity`,
          cell: (row) =>
            row.kind === "trade" ? (
              <Quantity value={row.trade.quantity.replace("-", "")} />
            ) : (
              <span className="dim">—</span>
            ),
        },
        {
          key: "amount",
          sort: (row) =>
            row.kind === "trade" ? toNumber(row.trade.estimated_base) : toNumber(row.cash.deposit),
          header: t`Amount`,
          className: "money",
          cellClass: (row) =>
            row.kind === "trade"
              ? row.trade.quantity.startsWith("-")
                ? "neg"
                : "pos"
              : row.cash.deposit === null
                ? "dim"
                : "pos",
          cell: (row) =>
            row.kind === "trade" ? (
              <Money value={row.trade.estimated_base} digits={0} signed tone={false} />
            ) : row.cash.deposit === null ? (
              "—"
            ) : (
              <Money value={row.cash.deposit} digits={0} signed tone={false} />
            ),
        },
        {
          key: "after",
          sort: (row) => toNumber(row.kind === "trade" ? row.trade.weight_after : row.cash.weight_after),
          header: t`Weight after`,
          className: "table-wide",
          cell: (row) => weightCell(row, taxonomy),
        },
      ]}
      foot={
        <tr>
          <td colSpan={6} className="dim">
            {plural(trades.length, { one: "# trade in the plan", other: "# trades in the plan" })}
            {cashRows.length > 0 &&
              ` · ${plural(cashRows.length, { one: "# cash row", other: "# cash rows" })}`}
          </td>
        </tr>
      }
    />
  );
}

/** The subject column: a security with its direction, or a cash account. */
function SubjectCell({ row, securities }: { row: PlanRow; securities: SecurityRow[] | undefined }) {
  const { t } = useLingui();
  if (row.kind === "cash") {
    return (
      <div className="instr">
        <Logo symbol="" name={row.cash.account_name} />
        <div>
          <div className="nm">{row.cash.account_name}</div>
          <div className="sub">
            <Badge tone="info">{t`CASH`}</Badge>
            <span>{row.cash.currency}</span>
          </div>
        </div>
      </div>
    );
  }
  const buy = !row.trade.quantity.startsWith("-");
  return (
    <SecurityLink id={row.trade.security_id} className="instr">
      <Logo symbol={row.trade.symbol} />
      <div>
        <div className="nm">{nameOf(securities, row.trade.security_id, row.trade.symbol)}</div>
        <div className="sub">
          <Badge tone={buy ? "in" : "out"}>{buy ? t`BUY` : t`SELL`}</Badge>
          <span>{row.trade.symbol}</span>
        </div>
      </div>
    </SecurityLink>
  );
}

function nodeCell(row: PlanRow, taxonomy: TaxonomyData | undefined) {
  const label = row.kind === "trade" ? row.item.label : row.cash.node;
  if (!label)
    return (
      <span className="pill pill--empty pill--flat">
        <Trans>unclassified</Trans>
      </span>
    );
  const slot = row.kind === "trade" ? (slotOfNode(taxonomy, row.item.node_id) ?? 1) : row.cash.slot;
  return (
    <span className="pill pill--flat">
      <Swatch slot={slot} />
      {label}
    </span>
  );
}

/** Weight after the trade, as a share of a 40 % full bar. */
function weightCell(row: PlanRow, taxonomy: TaxonomyData | undefined) {
  const after = row.kind === "trade" ? row.trade.weight_after : row.cash.weight_after;
  if (after === null) return <span className="dim">—</span>;
  const slot = row.kind === "trade" ? (slotOfNode(taxonomy, row.item.node_id) ?? 1) : row.cash.slot;
  return (
    <div className="tradeweight">
      <Percent value={after} digits={1} dim />
      {/* A 40 % target fills the track, so ordinary weights stay comparable. */}
      <Bar size="sm" slot={slot} fill={`${Math.min(100, (Number(after) / 0.4) * 100)}%`} />
    </div>
  );
}
