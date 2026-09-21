import { Trans, useLingui } from "@lingui/react/macro";
import type { UseQueryResult } from "@tanstack/react-query";
import { AllocationBar } from "../../components/domain/AllocationBar";
import { DriftBars, Sunburst, Treemap, TreemapTree } from "../../components/charts";
import { Empty, Pending, QueryError } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import type { Allocation, AllocationTarget, NodeMember, RebalancePlan, TaxonomyData } from "../../lib/types";
import { UNCLASSIFIED, bucketAt, descend, nodeSlotById, type LevelRow } from "./model";

/** The chart chosen by the view switch, with the loading state of its own query. */
export function BreakdownView({
  view,
  taxonomy,
  rows,
  currency,
  path,
  onPath,
  atPositions,
  target,
  members,
  tree,
  plan,
  nameOf,
  onEnter,
  onEnterTile,
}: {
  view: string;
  taxonomy: TaxonomyData;
  rows: LevelRow[];
  currency: string;
  path: string[];
  onPath: (path: string[]) => void;
  atPositions: boolean;
  target: AllocationTarget | null;
  members: UseQueryResult<NodeMember[]>;
  tree: UseQueryResult<Allocation>;
  plan: UseQueryResult<RebalancePlan>;
  nameOf: (key: string) => string | undefined;
  onEnter: (key: string) => void;
  onEnterTile: (key: string) => void;
}) {
  const { t } = useLingui();
  if (atPositions && members.isPending) return <Pending />;
  if (atPositions && members.isError) return <QueryError error={members.error} />;

  if (view === "target") {
    if (!target) {
      return (
        <Empty title={t`This classification has no target`}>
          <Trans>
            Give at least one category a weight: under "Edit tree" every category has a "Target weight" field.
            The drift and the trades that close it will appear here.
          </Trans>
        </Empty>
      );
    }
    if (plan.isPending) return <Pending />;
    if (plan.isError) return <QueryError error={plan.error} />;
    const levelDrift = plan.data.items.filter((item) => rows.some((r) => r.key === item.node_id));
    if (levelDrift.length === 0) {
      return (
        <Empty title={atPositions ? t`Instruments never carry a target` : t`No targets on this level`}>
          {plan.data.items.length > 0
            ? t`The target sits on other categories: go one level up, or into one that has it.`
            : t`Give at least one category on this level a weight — under "Edit tree" every category has a "Target weight" field.`}
        </Empty>
      );
    }
    return (
      <DriftBars
        items={levelDrift}
        currency={currency}
        relative
        slotOf={(id) => rows.find((r) => r.key === id)?.slot}
        onPick={onEnter}
      />
    );
  }

  if (view === "tree" || view === "rings") {
    if (tree.isPending) return <Pending />;
    if (tree.isError) return <QueryError error={tree.error} />;
    const buckets = descend(tree.data.buckets, path);
    const total = bucketAt(tree.data.buckets, path)?.value_base ?? tree.data.total_base;
    const slotOfKey = (key: string) => (key === UNCLASSIFIED ? 8 : nodeSlotById(taxonomy, key));
    return view === "rings" ? (
      <Sunburst
        buckets={buckets}
        currency={currency}
        total={total}
        slotOf={slotOfKey}
        nameOf={nameOf}
        onPick={onEnterTile}
        onUp={path.length > 0 ? () => onPath(path.slice(0, -1)) : undefined}
      />
    ) : (
      <TreemapTree
        buckets={buckets}
        currency={currency}
        slotOf={slotOfKey}
        nameOf={nameOf}
        onPick={onEnterTile}
      />
    );
  }

  if (view === "map") {
    return (
      <Treemap
        items={[...rows]
          .sort((a, b) => Number(b.weight) - Number(a.weight))
          .map((row) => ({
            key: row.key,
            label: row.label,
            weight: row.weight,
            value: formatMoney(row.value, currency, { compact: true }),
            slot: row.slot,
          }))}
        onPick={onEnter}
      />
    );
  }

  return (
    <AllocationBar
      items={rows.map((row) => ({
        key: row.key,
        label: row.label,
        value: row.value,
        weight: row.weight,
        slot: row.slot,
      }))}
      currency={currency}
      onPick={onEnter}
    />
  );
}
