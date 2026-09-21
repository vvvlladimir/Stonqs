import type { UseQueryResult } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { Calendar } from "../../components/charts";
import { Async, Legend, LegendItem, Money, Panel } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import type { IncomeData, MonthIncome, TransactionKind, YearIncome } from "../../lib/types";
import { kindNote, monthCell } from "./model";

/** Every month of history as a heat map; the window filter never narrows it. */
export function CalendarPanel({
  history,
  months,
  month,
  currency,
  kind,
  onPick,
}: {
  history: UseQueryResult<IncomeData>;
  months: MonthIncome[];
  month: MonthIncome | null;
  currency: string;
  kind: TransactionKind | null;
  onPick: (picked: { year: number; month: number }) => void;
}) {
  const { t, i18n } = useLingui();
  const all = history.data;
  return (
    <Panel
      title={t`When the money arrives`}
      note={all ? t`received, ${kindNote(i18n, kind)}, ${all.by_year.length} yr` : undefined}
      info={t`The right column is each year's total, the bottom row each month across all years; pick a cell to open that month.`}
      tools={all ? <Scale months={months} currency={currency} /> : undefined}
    >
      <Async query={history}>
        {(all) => (
          <>
            <Calendar
              full
              cells={months.map((m) => monthCell(i18n, m, currency))}
              totals={all.by_year.map((year: YearIncome) => ({
                year: year.year,
                text: formatMoney(year.net_base, currency, {
                  compact: true,
                  symbol: false,
                }),
              }))}
              footer={all.by_month_of_year.map((m) => ({
                month: m.month,
                text: formatMoney(m.net_base, currency, {
                  compact: true,
                  symbol: false,
                }),
              }))}
              footerTotal={formatMoney(all.total.net_base, currency, {
                compact: true,
                symbol: false,
              })}
              selected={month ? { year: month.year, month: month.month } : undefined}
              onPick={(year, m) => onPick({ year, month: m })}
            />
          </>
        )}
      </Async>
    </Panel>
  );
}

/** Calendar shading legend: five steps of the same colour plus the busiest month. */
function Scale({ months, currency }: { months: MonthIncome[]; currency: string }) {
  if (months.length === 0) return null;
  const peak = months.reduce((max, m) => (Number(m.net_base) > Number(max) ? m.net_base : max), "0");
  return (
    <Legend>
      <span className="dim">
        <Trans>less</Trans>
      </span>
      {[16, 32, 48, 64, 80].map((step) => (
        // eslint-disable-next-line lingui/no-unlocalized-strings -- a CSS colour expression
        <LegendItem key={step} color={`color-mix(in srgb, var(--pos) ${step}%, var(--surface-3))`} />
      ))}
      <Money value={peak} currency={currency} compact dim />
    </Legend>
  );
}
