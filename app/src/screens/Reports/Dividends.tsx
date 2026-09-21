import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { DataTable, Empty, Money, Num, Panel, Percent } from "../../components/ui";
import { Instrument } from "../../components/domain/Instrument";
import { formatDay, toNumber } from "../../lib/format";
import { DIVIDEND_FREQUENCY_LABELS } from "../../lib/kinds";
import type { ReportsData } from "../../lib/types";

/** Dividends as an accounting statement: gross, withholding and what actually arrived. */
export function Dividends({ data }: { data: ReportsData }) {
  const { t, i18n } = useLingui();
  const currency = data.base_currency;
  const total = data.dividends_total;

  if (total.payments === 0) {
    return (
      <Empty title={t`No dividends in this period`}>
        <Trans>
          The payment calendar and the split by payer live on the "Income" screen — together with account
          interest.
        </Trans>
      </Empty>
    );
  }

  return (
    <>
      {data.dividends_by_year.length > 1 && (
        <Panel title={t`By year`} table>
          <DataTable
            rows={data.dividends_by_year}
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
                key: "payments",
                sort: (row) => row.payments,
                header: t`Payments`,
                cell: (row) => <Num>{row.payments}</Num>,
              },
              {
                key: "gross",
                sort: (row) => toNumber(row.gross_base),
                header: t`Accrued`,
                only: "wide",
                cell: (row) => <Money value={row.gross_base} />,
              },
              {
                key: "taxes",
                sort: (row) => toNumber(row.taxes_base),
                header: t`Tax`,
                only: "wide",
                cell: (row) => <Money value={row.taxes_base} />,
              },
              {
                key: "net",
                sort: (row) => toNumber(row.net_base),
                header: t`Received, ${currency}`,
                card: "value",
                cell: (row) => <Money value={row.net_base} />,
              },
            ]}
          />
        </Panel>
      )}

      <Panel
        title={t`By instrument`}
        info={t`"Pays", "Yield a year" and "Yield on cost" use the instrument's whole payment history, not just the period.`}
        table
      >
        <DataTable
          rows={data.dividends_by_security}
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
              key: "payments",
              sort: (row) => row.payments,
              header: t`Payments`,
              only: "wide",
              cell: (row) => <Num>{row.payments}</Num>,
            },
            {
              key: "gross",
              sort: (row) => toNumber(row.gross_base),
              header: t`Accrued`,
              only: "wide",
              cell: (row) => <Money value={row.gross_base} />,
            },
            {
              key: "taxes",
              sort: (row) => toNumber(row.taxes_base),
              header: t`Tax`,
              only: "wide",
              cell: (row) => <Money value={row.taxes_base} />,
            },
            {
              key: "freq",
              sort: (row) => {
                const label = DIVIDEND_FREQUENCY_LABELS[row.frequency];
                return label ? i18n._(label) : null;
              },
              header: t`Pays`,
              align: "left",
              only: "wide",
              cell: (row) => {
                const label = DIVIDEND_FREQUENCY_LABELS[row.frequency];
                return label ? i18n._(label) : "—";
              },
            },
            {
              key: "last",
              sort: (row) => row.last_payment,
              header: t`Last payment`,
              only: "wide",
              cell: (row) => (row.last_payment === null ? "—" : formatDay(row.last_payment)),
            },
            {
              key: "annual",
              sort: (row) => toNumber(row.annual_yield_on_cost),
              header: t`Yield a year`,
              factLabel: t`yield a year`,
              cell: (row) =>
                row.annual_yield_on_cost === null ? (
                  "—"
                ) : (
                  <Percent value={row.annual_yield_on_cost} digits={2} />
                ),
            },
            {
              key: "yoc",
              sort: (row) => toNumber(row.yield_on_cost),
              header: t`Yield on cost`,
              factLabel: t`yield on cost`,
              only: "wide",
              cell: (row) =>
                row.yield_on_cost === null ? "—" : <Percent value={row.yield_on_cost} digits={2} />,
            },
            {
              key: "net",
              sort: (row) => toNumber(row.net_base),
              header: t`Received, ${currency}`,
              card: "value",
              cell: (row) => <Money value={row.net_base} />,
              foot: <Money value={total.net_base} />,
            },
          ]}
        />
      </Panel>

      <Panel
        title={t`Payments`}
        note={plural(data.payments.length, { one: "# payment", other: "# payments" })}
        table
      >
        <DataTable
          rows={data.payments.map((row, index) => ({ ...row, index }))}
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
              key: "security",
              sort: (row) => row.name || row.symbol,
              header: t`Instrument`,
              align: "left",
              card: "title",
              cell: (row) =>
                row.symbol ? (
                  <Instrument id={row.security_id} symbol={row.symbol} name={row.name} />
                ) : (
                  <span className="muted">
                    <Trans>no instrument</Trans>
                  </span>
                ),
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
              key: "gross_ccy",
              sort: (row) => toNumber(row.gross_in_currency),
              header: t`Accrued in currency`,
              only: "wide",
              cell: (row) => <Money value={row.gross_in_currency} currency={row.currency} />,
            },
            {
              key: "gross",
              sort: (row) => toNumber(row.gross_base),
              header: t`Accrued`,
              factLabel: t`accrued`,
              cell: (row) => <Money value={row.gross_base} />,
            },
            {
              key: "taxes",
              sort: (row) => toNumber(row.taxes_base),
              header: t`Tax`,
              factLabel: t`tax`,
              cell: (row) => <Money value={row.taxes_base} />,
            },
            {
              key: "net",
              sort: (row) => toNumber(row.net_base),
              header: t`Received, ${currency}`,
              card: "value",
              cell: (row) => <Money value={row.net_base} />,
            },
          ]}
        />
      </Panel>
    </>
  );
}
