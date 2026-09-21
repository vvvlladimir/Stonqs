import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { CaretLeftIcon, CaretRightIcon } from "@phosphor-icons/react";
import { Metric, Metrics, Money, Panel } from "../../components/ui";
import { formatMonthName, formatPercent } from "../../lib/format";
import type { IncomeEvent, MonthIncome } from "../../lib/types";
import { Events } from "./Events";
import { ratio } from "./model";

/** One month opened from the calendar: its totals, the year-ago figure and its payments. */
export function MonthPanel({
  month,
  months,
  events,
  currency,
  onStep,
}: {
  month: MonthIncome;
  months: MonthIncome[];
  events: IncomeEvent[];
  currency: string;
  onStep: (picked: { year: number; month: number }) => void;
}) {
  const { t } = useLingui();
  const index = months.findIndex((m) => m.year === month.year && m.month === month.month);
  const yearAgo = months.find((m) => m.year === month.year - 1 && m.month === month.month);
  const key = `${month.year}-${String(month.month).padStart(2, "0")}`;
  const rows = events.filter((event) => event.date.startsWith(key));

  return (
    <Panel
      title={
        <span className="monthnav">
          <button
            type="button"
            className="iconbtn iconbtn--sm"
            title={t`Previous month`}
            disabled={index <= 0}
            onClick={() =>
              onStep({
                year: months[index - 1].year,
                month: months[index - 1].month,
              })
            }
          >
            <CaretLeftIcon />
          </button>
          <span className="monthnav__t">
            {formatMonthName(month.month)} {month.year}
          </span>
          <button
            type="button"
            className="iconbtn iconbtn--sm"
            title={t`Next month`}
            disabled={index < 0 || index >= months.length - 1}
            onClick={() =>
              onStep({
                year: months[index + 1].year,
                month: months[index + 1].month,
              })
            }
          >
            <CaretRightIcon />
          </button>
        </span>
      }
      tools={
        <span className="panel__note">
          <Trans>
            <Money value={month.net_base} currency={currency} /> received
          </Trans>
        </span>
      }
    >
      {/* Panel-local tiles, so this row is not the page's metric slot. */}
      <Metrics inset>
        <Metric
          label={t`Accrued`}
          value={<Money value={month.gross_base} currency={currency} />}
          hint={plural(month.events, { one: "# payment", other: "# payments" })}
        />
        <Metric
          label={t`Tax withheld`}
          value={<Money value={month.taxes_base} currency={currency} />}
          hint={t`${formatPercent(ratio(month.taxes_base, month.gross_base))} of the accrued amount`}
        />
        <Metric
          label={t`Received`}
          value={<Money value={month.net_base} currency={currency} />}
          hint={t`arrived on the account`}
        />
        <Metric
          label={t`Same month a year ago`}
          value={yearAgo ? <Money value={yearAgo.net_base} currency={currency} /> : "—"}
          hint={yearAgo ? t`received` : t`no history`}
        />
      </Metrics>
      <Events events={rows} currency={currency} />
    </Panel>
  );
}
