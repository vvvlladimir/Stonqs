import { useLingui } from "@lingui/react/macro";
import type { UseQueryResult } from "@tanstack/react-query";
import { DriftBars } from "../../components/charts";
import { Async, Choice, Panel, Seg } from "../../components/ui";
import { slotOfNode } from "../../lib/taxonomy";
import type { AllocationTarget, RebalancePlan, TaxonomyData } from "../../lib/types";

export type Mode = "both" | "buy";

/** Which target is rebalanced, and whether an overweight may be sold. */
export function RebalanceControls({
  targets,
  selected,
  onTarget,
  mode,
  onMode,
}: {
  targets: AllocationTarget[];
  selected: string | null;
  onTarget: (id: string) => void;
  mode: Mode;
  onMode: (mode: Mode) => void;
}) {
  const { t } = useLingui();
  return (
    <>
      {targets.length > 1 && (
        <Choice
          wide
          label={t`Target`}
          value={selected ?? ""}
          onChange={onTarget}
          options={targets.map((target) => ({ value: target.id, label: target.name }))}
        />
      )}
      <Seg
        label={t`Mode`}
        value={mode}
        onChange={onMode}
        options={[
          { value: "both", label: t`Buy and sell` },
          { value: "buy", label: t`Buy only` },
        ]}
      />
    </>
  );
}

export function DriftPanel({
  plan,
  items,
  currency,
  taxonomy,
}: {
  plan: UseQueryResult<RebalancePlan>;
  items: RebalancePlan["items"];
  currency: string;
  taxonomy: TaxonomyData | undefined;
}) {
  const { t } = useLingui();
  return (
    <Panel
      title={t`Deviation from target`}
      info={t`The fill is the current weight and the tick the target; hatching marks an overweight, a dashed run a shortfall.`}
    >
      <Async query={plan}>
        {() => (
          <DriftBars items={items} currency={currency} slotOf={(nodeId) => slotOfNode(taxonomy, nodeId)} />
        )}
      </Async>
    </Panel>
  );
}
