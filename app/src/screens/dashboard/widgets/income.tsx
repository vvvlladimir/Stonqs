import { bucketLabel, slotOfNode } from "../../../lib/taxonomy";
import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { AllocationBar } from "../../../components/domain/AllocationBar";
import { Async } from "../../../components/ui";
import { Calendar, type CalendarCell } from "../../../components/charts";
import { useIncomeOver, useIncomeTaxonomy, useTaxonomies } from "../../../lib/queries";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { formatMonth, formatMoney } from "../../../lib/format";
import { periodOf, taxonomyOf, type WidgetProps, sourceOf } from "./model";

export function IncomeCalendarWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const { t } = useLingui();
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const income = useIncomeOver(range, null, source);

  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );

  return (
    <Async
      query={income}
      isEmpty={(data) => data.by_month.length === 0}
      empty={
        <p className="muted">
          <Trans>No payments in this period.</Trans>
        </p>
      }
    >
      {(data) => {
        const currency = data.base_currency;
        const cells: CalendarCell[] = data.by_month.map((month) => ({
          year: month.year,
          month: month.month,
          value: Number(month.net_base),
          text: formatMoney(month.net_base, currency, { compact: true, symbol: false }),
          title: t`${formatMonth(month.year, month.month)}: ${formatMoney(month.net_base, currency)} · ${plural(month.events, { one: "# payment", other: "# payments" })}`,
        }));
        return (
          <Calendar
            height="fill"
            cells={cells}
            totals={data.by_year.map((year) => ({
              year: year.year,
              text: formatMoney(year.net_base, currency, { compact: true, symbol: false }),
            }))}
          />
        );
      }}
    </Async>
  );
}

/** Income by the tree's top level; categories that paid nothing are left out. */
export function IncomeTaxonomyWidget({ widget, date, period }: WidgetProps) {
  const source = sourceOf(widget);
  const { i18n } = useLingui();
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));
  const taxonomies = useTaxonomies();
  const chosen = taxonomyOf(widget.cfg) ?? taxonomies.data?.[0]?.id ?? null;
  const income = useIncomeTaxonomy(chosen, range, null, source);
  const taxonomy = taxonomies.data?.find((item) => item.id === chosen);

  if (!range || (taxonomies.isSuccess && chosen === null))
    return (
      <p className="muted">
        <Trans>No classification has been set up yet.</Trans>
      </p>
    );

  return (
    <Async
      query={income}
      isEmpty={(data) => data.total.events === 0}
      empty={
        <p className="muted">
          <Trans>No payments in this period.</Trans>
        </p>
      }
    >
      {(data) => (
        <AllocationBar
          items={data.nodes
            .filter((node) => node.summary.events > 0)
            .map((node) => ({
              key: node.key,
              label: bucketLabel(i18n, node),
              value: node.summary.net_base,
              weight: node.weight,
              slot: slotOfNode(taxonomy, node.key),
            }))}
          total={data.total.net_base}
          currency={data.base_currency}
          limit={Number(widget.cfg.count) || 8}
          tracks={false}
        />
      )}
    </Async>
  );
}
