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
import {
  ErrorText,
  Metric,
  Metrics,
  Money,
  Panel,
  Pending,
  Percent,
  QueryError,
  Seg,
} from "../../components/ui";
import type { Asset, AssetInput } from "../../lib/types";
import { AssetForm } from "./AssetForm";
import { ValueHistory } from "./ValueHistory";
import { Side } from "./Sides";
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
  const add = (owed: boolean) =>
    setDraft(blankAsset(owed ? "MORTGAGE" : "PROPERTY", currency, today()));
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
      metrics={
        <Metrics>
          <Metric
            label={t`Net worth`}
            value={<Money value={reading.net_base} currency={currency} />}
            hint={t`everything owned, minus everything owed`}
            tip={t`A balance sheet, not a performance: nothing here has a return.`}
          />
          <Metric
            label={t`Invested`}
            value={<Money value={reading.investments_base} currency={currency} />}
            hint={
              reading.invested_share !== null ? (
                <Trans>
                  <Percent value={reading.invested_share} digits={0} /> of net worth
                </Trans>
              ) : undefined
            }
            tip={t`The portfolio's own value — the figure every other screen measures.`}
          />
          <Metric
            label={t`Other assets`}
            value={<Money value={reading.owned_base} currency={currency} />}
          />
          <Metric
            label={t`Debts`}
            value={<Money value={reading.owed_base} currency={currency} />}
            tone={Number(reading.owed_base) > 0 ? "negative" : "neutral"}
          />
        </Metrics>
      }
    >
      <ErrorText error={remove.error} />

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
    </Page>
  );
}
