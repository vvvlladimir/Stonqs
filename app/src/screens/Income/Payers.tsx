import { Trans, useLingui } from "@lingui/react/macro";
import { Instrument } from "../../components/domain/Instrument";
import { DataTable, Money, Percent } from "../../components/ui";
import { toNumber } from "../../lib/format";
import type { IncomeData } from "../../lib/types";
import { WIDTH, ratio } from "./model";

export function Payers({ data, value }: { data: IncomeData; value: Map<string, string> }) {
  const { t } = useLingui();
  if (data.by_security.length === 0)
    return (
      <p className="muted">
        <Trans>No payments from instruments.</Trans>
      </p>
    );

  const rows = [...data.by_security].sort((a, b) => Number(b.net_base) - Number(a.net_base));
  const currency = data.base_currency;

  return (
    <DataTable
      className="paytab paytab--payers"
      fixed
      card={false}
      rows={rows}
      rowKey={(row) => row.security_id}
      columns={[
        {
          key: "payer",
          sort: (row) => row.name || row.symbol,
          header: t`Payer`,
          align: "left",
          cell: (row) => <Instrument id={row.security_id} symbol={row.symbol} name={row.name} />,
        },
        {
          key: "net",
          sort: (row) => toNumber(row.net_base),
          header: t`Received, ${currency}`,
          width: WIDTH.payer,
          cell: (row) => <Money value={row.net_base} />,
        },
        {
          key: "share",
          sort: (row) => toNumber(row.net_base),
          header: t`Share`,
          only: "wide",
          width: WIDTH.share,
          className: "share",
          cell: (row) => (
            <Percent value={ratio(row.net_base, data.total.net_base)} digits={1} dim className="share__pct" />
          ),
        },
        {
          key: "yield",
          sort: (row) =>
            toNumber(value.get(row.security_id) ? ratio(row.gross_base, value.get(row.security_id)!) : null),
          header: t`Yield`,
          width: WIDTH.payer,
          // Yield needs a market value; a closed position has nothing to divide by.
          cell: (row) => {
            const market = value.get(row.security_id);
            return market ? <Percent value={ratio(row.gross_base, market)} digits={2} /> : "—";
          },
        },
      ]}
    />
  );
}
