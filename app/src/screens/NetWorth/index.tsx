import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api, today } from "../../lib/api";
import { useAsOf } from "../../lib/asOf";
import { formatMoney } from "../../lib/format";
import { affects, useInvalidate, useNetWorth, useNetWorthSeries } from "../../lib/queries";
import { Command } from "../../lib/commands";
import { Page } from "../../components/Page";
import { NetWorthChart } from "../../components/charts";
import { Banner, ErrorText, Panel, Pending, QueryError, Seg } from "../../components/ui";
import type { Asset, AssetInput } from "../../lib/types";
import { AssetForm } from "./AssetForm";
import { ValuesFileDialog } from "./ValuesFileDialog";
import { useValuesFile } from "./useValuesFile";
import { ValueHistory } from "./ValueHistory";
import { Side } from "./Sides";
import { Totals } from "./Totals";
import { assetToInput, blankAsset, windowFrom } from "./model";

/** Years of line the chart draws back from the reading date. */
type Window = "1" | "5";

/**
 * Everything owned and owed. A second total beside the portfolio's own, never a new value for it
 * (ADR-0092), and never narrowed by the account picker — an asset is not an account.
 */
export function NetWorth() {
  const { t } = useLingui();
  const date = useAsOf().date;
  const invalidate = useInvalidate();
  const data = useNetWorth(date);
  const [years, setYears] = useState<Window>("1");
  const line = useNetWorthSeries(windowFrom(date, Number(years)), date);
  const [draft, setDraft] = useState<AssetInput | null>(null);
  const [history, setHistory] = useState<Asset | null>(null);
  const csv = useValuesFile();

  const save = useMutation({
    mutationFn: api.assetSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.assets);
    },
  });
  const remove = useMutation({
    mutationFn: api.assetDelete,
    onSuccess: () => invalidate(...affects.assets),
  });

  if (data.isError) return <QueryError error={data.error} />;
  if (!data.data) return <Pending />;

  const { reading, assets } = data.data;
  const currency = reading.base_currency;
  const add = (owed: boolean) => setDraft(blankAsset(owed ? "MORTGAGE" : "PROPERTY", currency, today()));
  const del = (asset: Asset) => {
    if (confirm(t`Delete "${asset.name}"? Its valuations go with it; nothing else changes.`)) {
      remove.mutate(asset.id);
    }
  };

  return (
    <Page
      archetype="analysis"
      title={t`Net worth`}
      summary={t`${formatMoney(reading.net_base, currency)} in total, of which ${formatMoney(reading.investments_base, currency)} is invested`}
      actions={
        <>
          <button className="btn" onClick={() => add(false)}>
            <Trans>Add asset</Trans>
          </button>
          <button className="btn btn--ghost" onClick={csv.pick}>
            <Trans>Import valuations…</Trans>
          </button>
          <button className="btn btn--ghost" onClick={csv.exportAll}>
            <Trans>Export</Trans>
          </button>
          <Command id="new" label={t`Add asset`} run={() => add(false)} />
        </>
      }
      controls={
        <Seg
          label={t`Window`}
          value={years}
          onChange={setYears}
          options={[
            { value: "1", label: t`1 year` },
            { value: "5", label: t`5 years` },
          ]}
        />
      }
      banner={
        reading.stale_count > 0 ? (
          // Not an error and not fixable by the app: only the owner knows whether the figure holds.
          <Banner tone="info">
            {plural(reading.stale_count, {
              one: "# figure here has not been revisited for more than half a year",
              other: "# figures here have not been revisited for more than half a year",
            })}
          </Banner>
        ) : undefined
      }
      metrics={<Totals reading={reading} />}
    >
      <ErrorText error={remove.error} />
      {csv.error !== null && <Banner tone="warn">{csv.error}</Banner>}

      <Panel
        title={t`Over time`}
        info={t`The asset side steps between valuations: between two of them nothing was measured.`}
      >
        {line.data ? <NetWorthChart series={line.data} currency={currency} /> : <Pending />}
      </Panel>

      {(["owned", "owed"] as const).map((side) => (
        <Side
          key={side}
          side={side}
          reading={reading}
          assets={assets}
          onAdd={() => add(side === "owed")}
          onValues={setHistory}
          onEdit={(asset) => setDraft(assetToInput(asset))}
          onDelete={del}
        />
      ))}

      {draft && (
        <AssetForm
          draft={draft}
          assets={assets}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onClose={() => setDraft(null)}
          busy={save.isPending}
          error={save.error}
        />
      )}
      {history && <ValueHistory asset={history} onClose={() => setHistory(null)} />}
      {csv.file && (
        <ValuesFileDialog
          preview={csv.file.preview}
          onWrite={csv.write}
          onClose={csv.close}
          writing={csv.writing}
        />
      )}
    </Page>
  );
}
