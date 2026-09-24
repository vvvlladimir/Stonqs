import { Trans, useLingui } from "@lingui/react/macro";
import { DataTable, Legend, LegendItem, Money, Num, ShareBar, Swatch } from "../../components/ui";
import { formatMoney, toNumber } from "../../lib/format";
import { transactionLabel } from "../../lib/kinds";
import type { IncomeData, TransactionKind, YearKindIncome } from "../../lib/types";
import { WIDTH, kindSlots, ratio, share } from "./model";

export function KindLegend({ kinds }: { kinds: Array<{ kind: TransactionKind; slot: number }> }) {
  const { i18n } = useLingui();
  return (
    <Legend>
      {kinds.map(({ kind, slot }) => (
        <LegendItem key={kind} slot={slot}>
          {transactionLabel(i18n, kind)}
        </LegendItem>
      ))}
    </Legend>
  );
}

/** Year bars split by kind, then the same split as a table for the chosen window. */
export function Composition({
  data,
  all,
  currency,
}: {
  data: IncomeData;
  all: IncomeData;
  currency: string;
}) {
  const { t, i18n } = useLingui();
  const slots = new Map(kindSlots(all).map((k) => [k.kind, k.slot]));
  const peakYear = all.by_year.reduce((max, y) => (Number(y.net_base) > Number(max) ? y.net_base : max), "0");
  const partsOf = (year: number): YearKindIncome[] =>
    all.by_year_kind.filter((row) => row.year === year && Number(row.net_base) > 0);

  return (
    <>
      <div className="mix">
        {all.by_year.map((year) => (
          <div className="mix__row" key={year.year}>
            <Num dim>’{String(year.year).slice(2)}</Num>
            <ShareBar
              size="lg"
              legend={false}
              width={share(year.net_base, peakYear)}
              slices={partsOf(year.year).map((part) => ({
                key: part.kind,
                label: transactionLabel(i18n, part.kind),
                slot: slots.get(part.kind),
                share: ratio(part.net_base, year.net_base),
                // Money, not a share: the reader is comparing years, and 62 % of a bad year is
                // not a figure anyone wants back.
                tip: t`${transactionLabel(i18n, part.kind)} ${year.year}: ${formatMoney(part.net_base, currency)}`,
              }))}
            />
            <Money value={year.net_base} currency={currency} compact />
          </div>
        ))}
      </div>

      <DataTable
        className="paytab paytab--kinds"
        fixed
        card={false}
        rows={data.by_kind}
        rowKey={(kind) => kind.kind}
        columns={[
          {
            key: "kind",
            sort: (kind) => transactionLabel(i18n, kind.kind),
            header: t`In the window, ${currency}`,
            align: "left",
            foot: (
              <b>
                <Trans>Total</Trans>
              </b>
            ),
            cell: (kind) => (
              <span className="pill pill--flat">
                <Swatch slot={slots.get(kind.kind)} />
                <span>{transactionLabel(i18n, kind.kind)}</span>
              </span>
            ),
          },
          {
            key: "gross",
            sort: (kind) => toNumber(kind.gross_base),
            header: t`Accrued`,
            only: "wide",
            width: WIDTH.money,
            cell: (kind) => <Money value={kind.gross_base} />,
            foot: <Money value={data.total.gross_base} />,
          },
          {
            key: "taxes",
            sort: (kind) => toNumber(kind.taxes_base),
            header: t`Tax`,
            only: "wide",
            width: WIDTH.money,
            cell: (kind) => <Money value={kind.taxes_base} />,
            foot: <Money value={data.total.taxes_base} />,
          },
          {
            key: "net",
            sort: (kind) => toNumber(kind.net_base),
            header: t`Received`,
            width: WIDTH.money,
            cell: (kind) => <Money value={kind.net_base} />,
            foot: (
              <b>
                <Money value={data.total.net_base} />
              </b>
            ),
          },
          {
            key: "events",
            sort: (kind) => kind.events,
            header: t`Payments`,
            width: WIDTH.count,
            cell: (kind) => <Num dim>{kind.events}</Num>,
            foot: <Num dim>{data.total.events}</Num>,
          },
        ]}
      />
    </>
  );
}
