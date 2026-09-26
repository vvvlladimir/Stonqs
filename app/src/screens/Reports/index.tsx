import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../../lib/api";
import { exportNote } from "../../lib/legal";
import { useReports } from "../../lib/queries";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import { Page } from "../../components/Page";
import {
  Banner,
  Metric,
  Metrics,
  Money,
  Pending,
  Percent,
  QueryError,
  Seg,
  useMenu,
} from "../../components/ui";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { formatDay, formatMoney, signOf } from "../../lib/format";
import type { ReportsData } from "../../lib/types";
import { Charges } from "./Charges";
import { Dividends } from "./Dividends";
import { Gains } from "./Gains";
import { exportsOf, tabs, type Tab } from "./model";
import { useAsOf } from "../../lib/asOf";

/**
 * The tax-and-accounting view of a period: what was realized, what was paid out and what
 * it cost. Every table is a CSV away from a tax return, so the export follows the tab.
 */
export function Reports() {
  const { t, i18n } = useLingui();
  const asOf = useAsOf().date;
  const [period, setPeriod] = useState<PeriodId>("YTD");
  const [tab, setTab] = useState<Tab>("gains");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const menu = useMenu();

  const ranges = usePeriodRanges(asOf);
  const range = pickRange(ranges.data, period);
  const reports = useReports(range);

  if (ranges.isError) return <QueryError error={ranges.error} />;
  if (reports.isError) return <QueryError error={reports.error} />;
  if (ranges.isPending) return <Pending />;
  if (!range)
    return (
      <p className="muted">
        <Trans>No transactions — there is nothing to report yet.</Trans>
      </p>
    );
  if (reports.isPending) return <Pending />;

  const data = reports.data;

  const exportCsv = async (section: string) => {
    const path = await save({
      defaultPath: `${section.replace(".", "-")}-${data.from}_${data.to}.csv`,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    setSaving(true);
    setSaveError(null);
    try {
      await api.reportSave(section, data.from, data.to, path, exportNote());
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Page
      archetype="analysis"
      title={t`Reports`}
      controls={
        <>
          <Seg label={t`Report`} value={tab} onChange={setTab} options={tabs(i18n)} />
          <PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />
        </>
      }
      actions={
        <button
          type="button"
          className="btn btn--ghost"
          disabled={saving}
          aria-haspopup="menu"
          onClick={(e) =>
            menu.openFrom(
              "export",
              exportsOf(i18n)[tab].map((item) => ({
                label: item.label,
                onSelect: () => void exportCsv(item.section),
              })),
              e.currentTarget,
            )
          }
        >
          {saving ? t`Saving…` : t`Export CSV`}
        </button>
      }
      banner={
        saveError ? (
          <Banner tone="bad">
            <Trans>Could not save the file: {saveError}</Trans>
          </Banner>
        ) : undefined
      }
      metrics={<Tiles data={data} tab={tab} />}
    >
      {tab === "gains" && <Gains data={data} />}
      {tab === "dividends" && <Dividends data={data} />}
      {tab === "charges" && <Charges data={data} />}
      {menu.node}
    </Page>
  );
}

/** The tab's four headline numbers; the change hint always compares equal-length windows. */
function Tiles({ data, tab }: { data: ReportsData; tab: Tab }) {
  const { t } = useLingui();
  const currency = data.base_currency;
  const window = `${formatDay(data.previous_from)} — ${formatDay(data.previous_to)}`;
  const versus = (change: string) =>
    t`${formatMoney(change, currency, { signed: true, compact: true })} versus ${window}`;

  if (tab === "gains") {
    return (
      <Metrics>
        <Metric
          label={t`Result`}
          value={<Money value={data.gains_total.gain_base} currency={currency} signed tone={false} />}
          hint={versus(data.gains_change_base)}
          tone={signOf(data.gains_total.gain_base)}
          tip={t`Proceeds minus cost basis, fees and taxes on disposals only.`}
        />
        <Metric
          label={t`Return`}
          value={
            data.gains_return_on_cost === null ? (
              "—"
            ) : (
              <Percent value={data.gains_return_on_cost} digits={1} signed tone={false} />
            )
          }
          hint={t`against a cost basis of ${formatMoney(data.gains_total.cost_base, currency, { compact: true })}`}
          tone={data.gains_return_on_cost === null ? "neutral" : signOf(data.gains_return_on_cost)}
          tip={t`The result against the cost of what was sold.`}
        />
        <Metric
          label={t`Proceeds`}
          value={<Money value={data.gains_total.proceeds_base} currency={currency} />}
          hint={t`${formatMoney(data.gains_total.taxes_base, currency, { compact: true })} of tax withheld`}
        />
        <Metric
          label={t`Disposals`}
          value={String(data.gains_total.disposals)}
          hint={t`was ${data.gains_previous.disposals}`}
        />
      </Metrics>
    );
  }

  if (tab === "dividends") {
    return (
      <Metrics>
        <Metric
          label={t`Received`}
          value={<Money value={data.dividends_total.net_base} currency={currency} />}
          hint={versus(data.dividends_change_base)}
          tone={signOf(data.dividends_change_base)}
          tip={t`What reached the cash accounts, net of withholding tax.`}
        />
        <Metric
          label={t`Accrued`}
          value={<Money value={data.dividends_total.gross_base} currency={currency} />}
        />
        <Metric
          label={t`Tax withheld`}
          value={<Money value={data.dividends_total.taxes_base} currency={currency} />}
          tip={t`Withheld at the source, so it never reaches the account.`}
        />
        <Metric
          label={t`Payments`}
          value={String(data.dividends_total.payments)}
          hint={t`was ${data.dividends_previous.payments}`}
        />
      </Metrics>
    );
  }

  return (
    <Metrics>
      <Metric
        label={t`Total costs`}
        value={<Money value={data.charges_total_base} currency={currency} />}
        hint={versus(data.charges_change_base)}
        // Growing costs are bad news, so the tone is inverted against the money's own sign.
        tone={signOf(data.charges_change_base) === "positive" ? "negative" : "positive"}
        tip={t`Standalone fees and taxes only, never a commission inside a trade.`}
      />
      <Metric label={t`Fees`} value={<Money value={data.charges_total.fees_base} currency={currency} />} />
      <Metric label={t`Taxes`} value={<Money value={data.charges_total.taxes_base} currency={currency} />} />
      <Metric
        label={t`Transactions`}
        value={String(data.charges_total.count)}
        hint={t`was ${data.charges_previous.count}`}
      />
    </Metrics>
  );
}
