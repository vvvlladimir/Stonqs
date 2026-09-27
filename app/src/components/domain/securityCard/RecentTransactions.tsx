import { Trans, useLingui } from "@lingui/react/macro";
import { useTransactions } from "../../../lib/queries";
import { DataTable, Money, Panel, Quantity, SkeletonRows } from "../../ui";
import { formatDay, signOf, toNumber } from "../../../lib/format";
import { KindTag } from "../KindTag";
import type { TransactionRow } from "../../../lib/types";
import { RECENT } from "./recent";

/** The latest operations in the instrument; the journal holds the rest. */
export function RecentTransactions({ securityId }: { securityId: string }) {
  const { t } = useLingui();
  const transactions = useTransactions({ security_id: securityId });
  const rows = transactions.data?.rows ?? [];
  return (
    <>
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
    </>
  );
}
