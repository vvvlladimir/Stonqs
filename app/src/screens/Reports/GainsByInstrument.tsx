import { useLingui } from "@lingui/react/macro";
import { DataTable, Money, Num, Panel, Percent } from "../../components/ui";
import { Instrument } from "../../components/domain/Instrument";
import { toNumber } from "../../lib/format";
import type { ReportsData } from "../../lib/types";

/** The realized result per instrument over the window. */
export function GainsByInstrument({ data }: { data: ReportsData }) {
  const { t } = useLingui();
  const currency = data.base_currency;
  const total = data.gains_total;
  return (
    <Panel
      title={t`By instrument`}
      info={t`The result is proceeds minus cost basis, fees and taxes; "Of it, FX" is the part made by the exchange rate.`}
      table
    >
      <DataTable
        rows={data.gains_by_security}
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
            key: "disposals",
            sort: (row) => row.disposals,
            header: t`Disposals`,
            only: "wide",
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
            foot: <Money value={total.gain_base} signed />,
          },
        ]}
      />
    </Panel>
  );
}
