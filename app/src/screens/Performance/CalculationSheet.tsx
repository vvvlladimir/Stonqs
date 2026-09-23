import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { Trans, useLingui } from "@lingui/react/macro";
import { msg } from "@lingui/core/macro";
import type { I18n } from "@lingui/core";

import { api } from "../../lib/api";
import { usePerformanceBreakdown } from "../../lib/queries";
import { formatMonth } from "../../lib/format";
import { toNumber } from "../../lib/format";
import { Async, Banner, Choice, DataTable, Money, Panel, Percent } from "../../components/ui";
import type { CalculationRow, PeriodRange, SheetPeriod } from "../../lib/types";

/** Day and week rows are offered only where they stay readable — a decade of days is not a table. */
const DAY_LIMIT = 92;
const WEEK_LIMIT = 400;

/**
 * The calculation sheet: one row per chunk, and the identity `start + flows + result = end`
 * running down it. Its footer is the same figure the metric strip shows, on purpose — seeing
 * the rows chain to it is what the panel is for.
 */
export function CalculationSheet({ range, currency }: { range: PeriodRange; currency: string }) {
  const { t, i18n } = useLingui();
  const [period, setPeriod] = useState<SheetPeriod>("MONTH");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const sheet = usePerformanceBreakdown(range, period);
  const days = spanInDays(range);

  const exportCsv = async () => {
    const path = await save({
      defaultPath: `calculation-${range.from}_${range.to}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    setSaving(true);
    setSaveError(null);
    try {
      await api.performanceSheetSave(range.from, range.to, period, path);
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  const options: { value: SheetPeriod; label: string }[] = [
    ...(days <= DAY_LIMIT ? [{ value: "DAY" as const, label: t`By day` }] : []),
    ...(days <= WEEK_LIMIT ? [{ value: "WEEK" as const, label: t`By week` }] : []),
    { value: "MONTH", label: t`By month` },
    { value: "QUARTER", label: t`By quarter` },
    { value: "YEAR", label: t`By year` },
  ];

  return (
    <Panel
      title={t`How the result adds up`}
      info={t`Each row opens where the one above it closed, so the money and the return can be followed step by step.`}
      table
      tools={
        <>
          <Choice
            label={t`Split the period by`}
            value={period}
            onChange={(value) => setPeriod(value as SheetPeriod)}
            options={options}
          />
          <button type="button" className="btn btn--ghost btn--sm" disabled={saving} onClick={exportCsv}>
            {saving ? t`Saving…` : t`Export CSV`}
          </button>
        </>
      }
    >
      {saveError && (
        <Banner tone="bad">
          <Trans>Could not save the file: {saveError}</Trans>
        </Banner>
      )}
      <Async query={sheet}>
        {(data) => (
          <DataTable
            rows={data.rows}
            rowKey={(row) => row.from}
            columns={[
              {
                key: "period",
                header: t`Period`,
                align: "left",
                cell: (row) => label(i18n, data.period, row),
                foot: <Trans>Whole period</Trans>,
              },
              {
                key: "start",
                header: t`Opening, ${currency}`,
                only: "wide",
                sort: (row) => toNumber(row.start_value_base),
                cell: (row) => <Money value={row.start_value_base} currency={currency} />,
                foot: <Money value={data.total.start_value_base} currency={currency} />,
              },
              {
                key: "flows",
                header: (
                  <span data-tip={t`Money paid in or taken out; it is not a gain.`}>
                    <Trans>Flows</Trans>
                  </span>
                ),
                sort: (row) => toNumber(row.external_flow_base),
                cell: (row) => <Money value={row.external_flow_base} currency={currency} signed />,
                foot: <Money value={data.total.net_flow_base} currency={currency} signed />,
              },
              {
                key: "market",
                header: t`Market`,
                only: "wide",
                sort: (row) => toNumber(row.market_change_base),
                cell: (row) => <Money value={row.market_change_base} currency={currency} signed />,
              },
              {
                key: "income",
                header: t`Income`,
                only: "wide",
                sort: (row) => toNumber(row.income_base),
                cell: (row) => <Money value={row.income_base} currency={currency} />,
              },
              {
                key: "costs",
                header: t`Costs`,
                only: "wide",
                sort: (row) => toNumber(row.costs_base),
                cell: (row) => <Money value={row.costs_base} currency={currency} />,
              },
              {
                key: "result",
                header: (
                  <span data-tip={t`The change with the flows taken out — what the chunk earned.`}>
                    <Trans>Result</Trans>
                  </span>
                ),
                sort: (row) => toNumber(row.delta_base),
                cell: (row) => <Money value={row.delta_base} currency={currency} signed />,
                foot: <Money value={data.total.delta_base} currency={currency} signed />,
              },
              {
                key: "end",
                header: t`Closing, ${currency}`,
                sort: (row) => toNumber(row.end_value_base),
                cell: (row) => <Money value={row.end_value_base} currency={currency} />,
                foot: <Money value={data.total.end_value_base} currency={currency} />,
              },
              {
                key: "twr",
                header: t`Return`,
                sort: (row) => toNumber(row.twr),
                cell: (row) => <Percent value={row.twr} signed />,
              },
              {
                key: "cumulative",
                header: (
                  <span data-tip={t`The rows so far, chained — the last one is the whole period.`}>
                    <Trans>Chained</Trans>
                  </span>
                ),
                only: "wide",
                sort: (row) => toNumber(row.cumulative_twr),
                cell: (row) => <Percent value={row.cumulative_twr} signed />,
                foot: <Percent value={data.twr} signed />,
              },
            ]}
          />
        )}
      </Async>
    </Panel>
  );
}

/** Calendar days the period covers; both ends count. */
function spanInDays(range: PeriodRange): number {
  const from = Date.parse(range.from);
  const to = Date.parse(range.to);
  return Math.round((to - from) / 86_400_000) + 1;
}

/** Names a chunk the way its granularity is read: a month by name, a quarter by number. */
function label(i18n: I18n, period: SheetPeriod, row: CalculationRow): string {
  const [year, month, day] = row.from.split("-").map(Number);
  switch (period) {
    case "YEAR":
      return String(year);
    case "QUARTER": {
      const quarter = Math.ceil(month / 3);
      return i18n._(msg`Q${quarter} ${year}`);
    }
    case "MONTH":
      return formatMonth(year, month);
    default:
      return new Date(Date.UTC(year, month - 1, day)).toLocaleDateString(i18n.locale, {
        timeZone: "UTC",
        day: "numeric",
        month: "short",
      });
  }
}
