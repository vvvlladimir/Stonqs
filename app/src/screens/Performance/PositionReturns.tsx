import { Trans, useLingui } from "@lingui/react/macro";
import { Instrument } from "../../components/domain/Instrument";
import { DataTable, Money, Percent } from "../../components/ui";
import { toNumber } from "../../lib/format";
import type { PositionReturnRow } from "../../lib/types";

export function PositionReturns({ rows, currency }: { rows: PositionReturnRow[]; currency: string }) {
  const { t } = useLingui();
  return (
    <DataTable
      variant="rows"
      rows={rows}
      rowKey={(row) => row.security_id}
      columns={[
        {
          key: "security",
          sort: (row) => row.name || row.symbol,
          header: t`Instrument`,
          align: "left",
          cell: (row) => <Instrument id={row.security_id} symbol={row.symbol} name={row.name} />,
        },
        {
          key: "twr",
          sort: (row) => toNumber(row.twr),
          header: <span data-tip={t`What the instrument did, regardless of when it was bought.`}>TWR</span>,
          factLabel: "TWR",
          cell: (row) => (row.twr ? <Percent value={row.twr} signed /> : "—"),
        },
        {
          key: "xirr",
          sort: (row) => toNumber(row.xirr),
          header: <span data-tip={t`Return that accounts for when the purchases happened.`}>IRR</span>,
          factLabel: "IRR",
          cell: (row) => (row.xirr ? <Percent value={row.xirr} signed /> : "—"),
        },
        {
          key: "pnl",
          sort: (row) => toNumber(row.pnl_base),
          header: t`Result, ${currency}`,
          factLabel: "",
          cell: (row) => <Money value={row.pnl_base} currency={currency} signed />,
        },
        {
          key: "contribution",
          sort: (row) => toNumber(row.contribution),
          header: (
            <span data-tip={t`Percentage points of the portfolio return that came from this instrument.`}>
              <Trans>Contribution</Trans>
            </span>
          ),
          cell: (row) => <Percent value={row.contribution} digits={2} signed />,
        },
      ]}
    />
  );
}
