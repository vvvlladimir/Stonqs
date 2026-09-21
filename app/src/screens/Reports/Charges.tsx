import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { DataTable, Empty, Money, Num, Panel } from "../../components/ui";
import { Instrument } from "../../components/domain/Instrument";
import { KindTag } from "../../components/domain/KindTag";
import { formatDay, toNumber } from "../../lib/format";
import type { ReportsData } from "../../lib/types";

/** Expenses are signed operations, so refunds reduce every sum here. */
export function Charges({ data }: { data: ReportsData }) {
  const { t } = useLingui();
  const currency = data.base_currency;

  if (data.charge_rows.length === 0) {
    return (
      <Empty title={t`No standalone fees or taxes in this period`}>
        <Trans>
          A commission inside a trade never reaches this table: it is already in the position's cost basis,
          and dividend tax sits next to its payment on the "Dividends" tab.
        </Trans>
      </Empty>
    );
  }

  return (
    <>
      {data.charges_by_year.length > 1 && (
        <Panel title={t`By year`} table>
          <DataTable
            rows={data.charges_by_year}
            rowKey={(row) => String(row.year)}
            columns={[
              {
                key: "year",
                sort: (row) => row.year,
                header: t`Year`,
                align: "left",
                className: "nm",
                cell: (row) => row.year,
              },
              {
                key: "count",
                sort: (row) => row.count,
                header: t`Transactions`,
                cell: (row) => <Num>{row.count}</Num>,
              },
              {
                key: "fees",
                sort: (row) => toNumber(row.fees_base),
                header: t`Fees`,
                only: "wide",
                cell: (row) => <Money value={row.fees_base} />,
              },
              {
                key: "taxes",
                sort: (row) => toNumber(row.taxes_base),
                header: t`Taxes`,
                only: "wide",
                cell: (row) => <Money value={row.taxes_base} />,
              },
              {
                key: "total",
                sort: (row) => toNumber(row.total_base),
                header: t`Total, ${currency}`,
                card: "value",
                cell: (row) => <Money value={row.total_base} />,
              },
            ]}
          />
        </Panel>
      )}

      <div className="grid-2 stack">
        <Panel title={t`By transaction kind`} table>
          <DataTable
            rows={data.charges_by_kind}
            rowKey={(row) => row.kind}
            columns={[
              {
                key: "kind",
                sort: (row) => row.kind,
                header: t`Kind`,
                align: "left",
                cell: (row) => <KindTag kind={row.kind} />,
              },
              {
                key: "count",
                sort: (row) => row.count,
                header: t`Transactions`,
                cell: (row) => <Num>{row.count}</Num>,
              },
              {
                key: "total",
                sort: (row) => toNumber(row.total_base),
                header: t`Total, ${currency}`,
                card: "value",
                cell: (row) => <Money value={row.total_base} />,
              },
            ]}
          />
        </Panel>

        <Panel
          title={t`By account`}
          info={t`Standalone fees and taxes by the account they were charged to.`}
          table
        >
          <DataTable
            rows={data.charges_by_account}
            rowKey={(row) => row.account_id}
            columns={[
              {
                key: "account",
                sort: (row) => row.account,
                header: t`Account`,
                align: "left",
                className: "nm",
                cell: (row) => row.account,
              },
              {
                key: "count",
                sort: (row) => row.count,
                header: t`Transactions`,
                cell: (row) => <Num>{row.count}</Num>,
              },
              {
                key: "total",
                sort: (row) => toNumber(row.total_base),
                header: t`Total, ${currency}`,
                card: "value",
                cell: (row) => <Money value={row.total_base} />,
              },
            ]}
          />
        </Panel>
      </div>

      <Panel
        title={t`Transactions`}
        note={plural(data.charge_rows.length, { one: "# transaction", other: "# transactions" })}
        table
      >
        <DataTable
          rows={data.charge_rows.map((row, index) => ({ ...row, index }))}
          rowKey={(row) => String(row.index)}
          columns={[
            {
              key: "date",
              sort: (row) => row.date,
              header: t`Date`,
              align: "left",
              className: "nm",
              width: "7rem",
              cell: (row) => formatDay(row.date),
            },
            {
              key: "kind",
              sort: (row) => row.kind,
              header: t`Kind`,
              align: "left",
              card: "title",
              cell: (row) => <KindTag kind={row.kind} />,
            },
            {
              key: "account",
              sort: (row) => row.account,
              header: t`Account`,
              align: "left",
              only: "wide",
              cell: (row) => row.account,
            },
            {
              key: "security",
              sort: (row) => row.name || row.symbol,
              header: t`Instrument`,
              align: "left",
              only: "wide",
              cell: (row) =>
                row.symbol ? (
                  <Instrument id={row.security_id} symbol={row.symbol} name={row.name} />
                ) : (
                  <span className="muted">—</span>
                ),
            },
            {
              key: "amount_ccy",
              sort: (row) => toNumber(row.amount_in_currency),
              header: t`In currency`,
              factLabel: t`in currency`,
              cell: (row) => <Money value={row.amount_in_currency} currency={row.currency} />,
            },
            {
              key: "amount",
              sort: (row) => toNumber(row.amount_base),
              header: t`Amount, ${currency}`,
              card: "value",
              cell: (row) => <Money value={row.amount_base} />,
            },
          ]}
        />
      </Panel>
    </>
  );
}
