import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { DataTable, DayMark, Money, Panel, Percent, Quantity } from "../../components/ui";
import { Instrument } from "../../components/domain/Instrument";
import { KindTag } from "../../components/domain/KindTag";
import { toNumber } from "../../lib/format";
import type { ReportsData } from "../../lib/types";

/** Every sale or outbound delivery with its own result. */
export function Disposals({ data }: { data: ReportsData }) {
  const { t } = useLingui();
  const currency = data.base_currency;
  return (
    <Panel
      title={t`Trades`}
      note={plural(data.disposals.length, { one: "# disposal", other: "# disposals" })}
      table
    >
      <DataTable
        rows={data.disposals.map((row, index) => ({ ...row, index }))}
        rowKey={(row) => String(row.index)}
        columns={[
          {
            key: "date",
            sort: (row) => row.date,
            header: t`Date`,
            align: "left",
            className: "nm",
            width: "3.5rem",
            cell: (row) => <DayMark date={row.date} />,
          },
          {
            key: "security",
            sort: (row) => row.name || row.symbol,
            header: t`Instrument`,
            align: "left",
            card: "title",
            cell: (row) => <Instrument id={row.security_id} symbol={row.symbol} name={row.name} />,
          },
          {
            key: "kind",
            sort: (row) => row.kind,
            header: t`Transaction`,
            align: "left",
            only: "wide",
            cell: (row) => <KindTag kind={row.kind} />,
          },
          {
            key: "qty",
            sort: (row) => toNumber(row.quantity),
            header: t`Qty`,
            cell: (row) => <Quantity value={row.quantity} />,
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
