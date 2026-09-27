import { useLingui } from "@lingui/react/macro";
import { DataTable, Money, Num, Panel, Percent } from "../../components/ui";
import { toNumber } from "../../lib/format";
import type { ReportsData } from "../../lib/types";

/** The realized result summed per calendar year. */
export function GainsByYear({ data }: { data: ReportsData }) {
  const { t } = useLingui();
  const currency = data.base_currency;
  return (
    <Panel title={t`By year`} table>
      <DataTable
        rows={data.gains_by_year}
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
            key: "disposals",
            sort: (row) => row.disposals,
            header: t`Disposals`,
            cell: (row) => <Num>{row.disposals}</Num>,
          },
          {
            key: "proceeds",
            sort: (row) => toNumber(row.proceeds_base),
            header: t`Proceeds`,
            only: "wide",
            cell: (row) => <Money value={row.proceeds_base} />,
          },
          {
            key: "cost",
            sort: (row) => toNumber(row.cost_base),
            header: t`Cost basis`,
            only: "wide",
            cell: (row) => <Money value={row.cost_base} />,
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
            key: "fx",
            sort: (row) => toNumber(row.currency_gain_base),
            header: t`Of it, FX`,
            only: "wide",
            cell: (row) => <Money value={row.currency_gain_base} signed />,
          },
          {
            key: "roc",
            sort: (row) => toNumber(row.return_on_cost),
            header: t`Return`,
            factLabel: t`return`,
            cell: (row) =>
              row.return_on_cost === null ? "—" : <Percent value={row.return_on_cost} digits={1} signed />,
          },
          {
            key: "gain",
            sort: (row) => toNumber(row.gain_base),
            header: t`Result, ${currency}`,
            card: "value",
            cell: (row) => <Money value={row.gain_base} signed />,
          },
        ]}
      />
    </Panel>
  );
}
