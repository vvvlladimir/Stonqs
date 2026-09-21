import type { ReactNode } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Async, DataTable, Money, Panel, Scrolly, Seg } from "../../components/ui";
import { Instrument } from "../../components/domain/Instrument";
import { PAYMENT_LINE_LABELS } from "../../lib/kinds";
import { toNumber } from "../../lib/format";
import type { UseQueryResult } from "@tanstack/react-query";
import type { PaymentPeriod, PaymentsData } from "../../lib/types";
import { bucketLabel, paymentPeriods } from "./model";

/** One line of the grid, whichever table it belongs to. */
interface GridRow {
  key: string;
  label: ReactNode;
  /** The label as text, for ordering: a label may be a whole component. */
  name?: string;
  amounts: string[];
  total: string;
  /** A subtotal rather than a component of it. */
  strong?: boolean;
  /** The first subtotal: the rule above it is what separates the lines from their sum. */
  opens?: boolean;
}

/**
 * The payments grid: every dated line of the window against one axis. Income is split by kind
 * and by payer, and what the portfolio paid in and what its sales returned sit in the same
 * grid — the comparison is the point, so summing two separate reports would not do.
 */
export function PaymentsPanel({
  query,
  period,
  onPeriod,
}: {
  query: UseQueryResult<PaymentsData>;
  period: PaymentPeriod;
  onPeriod: (period: PaymentPeriod) => void;
}) {
  const { t, i18n } = useLingui();

  return (
    <Async query={query}>
      {(data) => {
        const currency = data.base_currency;
        // A bucket with no payment is dimmed rather than dropped: the axis is the point, and a
        // column of zeros no one reads is what made the grid hard to scan.
        const amount = (value: string) => (
          <Money value={value} signed tone={false} dim={toNumber(value) === 0} />
        );

        const columns = (label: string, sortable = false) => [
          {
            key: "line",
            header: label,
            align: "left" as const,
            width: "16rem",
            card: "title" as const,
            // `Instrument` cuts its own name; the two-line clamp would cut its logo instead.
            clamp: false as const,
            sort: sortable ? (row: GridRow) => row.name : undefined,
            cell: (row: GridRow) => row.label,
          },
          ...data.buckets.map((bucket, at) => ({
            key: `${bucket.year}-${bucket.index}`,
            header: bucketLabel(i18n, data.period, bucket),
            width: "6.5rem",
            sort: sortable ? (row: GridRow) => toNumber(row.amounts[at]) : undefined,
            cell: (row: GridRow) => amount(row.amounts[at]),
          })),
          {
            key: "total",
            header: t`Total, ${currency}`,
            width: "7.5rem",
            card: "value" as const,
            sort: sortable ? (row: GridRow) => toNumber(row.total) : undefined,
            cell: (row: GridRow) => amount(row.total),
          },
        ];

        const kinds: GridRow[] = [
          ...data.lines.map((row) => ({
            key: row.line,
            label: i18n._(PAYMENT_LINE_LABELS[row.line]),
            amounts: row.amounts,
            total: row.total,
          })),
          {
            key: "earnings",
            label: <Trans>Earned</Trans>,
            amounts: data.earnings,
            total: data.earnings_total,
            strong: true,
            opens: true,
          },
          {
            key: "cumulative",
            label: <Trans>Running total</Trans>,
            amounts: data.cumulative,
            total: data.earnings_total,
            strong: true,
          },
        ];

        const payers: GridRow[] = data.payers.map((payer) => ({
          key: payer.security_id ?? "cash",
          name: payer.security_id ? payer.name || payer.symbol : t`On the account`,
          label: payer.security_id ? (
            <Instrument id={payer.security_id} symbol={payer.symbol} name={payer.name} />
          ) : (
            <Trans>On the account</Trans>
          ),
          amounts: payer.amounts,
          total: payer.total,
        }));

        return (
          <>
            <Panel
              title={t`Summary`}
              info={t`"Earned" sums the income lines only; fees and money paid in are listed but not counted in it.`}
              tools={
                /* The control shows what was asked for, the axis what the core answered:
                   while a wider column is loading the two differ for one render. */
                <Seg
                  options={paymentPeriods(i18n)}
                  value={period}
                  onChange={onPeriod}
                  label={t`Column width`}
                />
              }
              table
            >
              <Scrolly x>
                <DataTable
                  rows={kinds}
                  rowKey={(row) => row.key}
                  rowProps={(row) => ({
                    className: row.strong ? `sum${row.opens ? " sum--first" : ""}` : undefined,
                  })}
                  sizing="content"
                  columns={columns(t`Line`)}
                />
              </Scrolly>
            </Panel>

            <Panel
              title={t`By payer over the same axis`}
              info={t`Income per paying instrument, in the same columns; fees and sales are not included.`}
              table
            >
              <Scrolly x>
                <DataTable
                  rows={payers}
                  rowKey={(row) => row.key}
                  sizing="content"
                  columns={columns(t`Payer`, true)}
                />
              </Scrolly>
            </Panel>
          </>
        );
      }}
    </Async>
  );
}
