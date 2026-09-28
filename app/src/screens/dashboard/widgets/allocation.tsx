import { bucketLabel } from "../../../lib/taxonomy";
import { Trans, useLingui } from "@lingui/react/macro";
import { AllocationBar } from "../../../components/domain/AllocationBar";
import { Async } from "../../../components/ui";
import { DriftBars, Treemap } from "../../../components/charts";
import { useAllocation, usePortfolio, useRebalance } from "../../../lib/queries";
import { formatMoney } from "../../../lib/format";
import { taxonomyOf, useWidgetTarget, type WidgetProps, sourceOf } from "./model";

export function TreemapWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const { i18n } = useLingui();
  const taxonomyId = taxonomyOf(widget.cfg);
  const allocation = useAllocation(taxonomyId ? "taxonomy" : "security", taxonomyId, date, source);
  const portfolio = usePortfolio();

  const currency = portfolio.data?.base_currency ?? "";

  return (
    <Async query={allocation}>
      {(data) => (
        <Treemap
          items={[...data.buckets]
            .sort((a, b) => Number(b.weight) - Number(a.weight))
            .map((bucket) => ({
              key: bucket.key,
              label: bucketLabel(i18n, bucket),
              weight: bucket.weight,
              value: formatMoney(bucket.value_base, currency, { compact: true }),
            }))}
          height="fill"
        />
      )}
    </Async>
  );
}

export function AllocationWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const { i18n } = useLingui();
  const taxonomyId = taxonomyOf(widget.cfg);
  const allocation = useAllocation(taxonomyId ? "taxonomy" : "security", taxonomyId, date, source);
  const portfolio = usePortfolio();

  return (
    <Async query={allocation}>
      {(data) => (
        <AllocationBar
          items={data.buckets.map((bucket) => ({
            key: bucket.key,
            label: bucketLabel(i18n, bucket),
            value: bucket.value_base,
            weight: bucket.weight,
          }))}
          currency={portfolio.data?.base_currency ?? ""}
          limit={Number(widget.cfg.count) || 8}
          tracks={false}
        />
      )}
    </Async>
  );
}

/** Where the portfolio stands against a target, as one bar per category. The plan itself
 * belongs to the rebalance screen; this only shows the drift that would call for it. */
export function DriftWidget({ widget, date }: WidgetProps) {
  const source = sourceOf(widget);
  const target = useWidgetTarget(widget);
  const portfolio = usePortfolio();
  const plan = useRebalance(target.id, date, null, true, source);

  if (target.id === null)
    return (
      <p className="muted">
        <Trans>No target has been set up yet.</Trans>
      </p>
    );

  return (
    <Async query={plan}>
      {(data) => <DriftBars items={data.items} currency={portfolio.data?.base_currency ?? ""} />}
    </Async>
  );
}
