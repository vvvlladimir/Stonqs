import { Trans, useLingui } from "@lingui/react/macro";
import { AllocationBar } from "../../components/domain/AllocationBar";
import { bucketLabel, slotOfNode } from "../../lib/taxonomy";
import type { TaxonomyData, TaxonomyIncomeData } from "../../lib/types";

/** Top-level roots with everything paid beneath; a split payer splits its payments. */
export function ByTaxonomy({
  data,
  taxonomy,
  currency,
}: {
  data: TaxonomyIncomeData;
  taxonomy: TaxonomyData | undefined;
  currency: string;
}) {
  const { i18n } = useLingui();
  // A node nothing paid into is not a zero row: the tree classifies value, this reads payments.
  const items = data.nodes
    .filter((node) => node.summary.events > 0)
    .map((node) => ({
      key: node.key,
      label: bucketLabel(i18n, node),
      value: node.summary.net_base,
      weight: node.weight,
      slot: slotOfNode(taxonomy, node.key),
    }));

  if (items.length === 0)
    return (
      <p className="muted">
        <Trans>No payments to classify in this period.</Trans>
      </p>
    );

  return <AllocationBar items={items} total={data.total.net_base} currency={currency} />;
}
