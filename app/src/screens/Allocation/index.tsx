import { bucketLabel } from "../../lib/taxonomy";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { CheckIcon, TreeStructureIcon } from "@phosphor-icons/react";
import { today } from "../../lib/api";
import { Page } from "../../components/Page";
import { Async, Empty, Pending, QueryError, Seg } from "../../components/ui";
import {
  affects,
  useAllocation,
  useAllocationMembers,
  useAllocationTree,
  useInvalidate,
  usePortfolio,
  usePositions,
  useRebalance,
  useSecurities,
  useTargets,
  useTaxonomies,
} from "../../lib/queries";
import { formatDay } from "../../lib/format";
import { slotFor } from "../../lib/plot";
import { BreakdownPanel } from "./BreakdownPanel";
import { useAllocationDialogs } from "./Dialogs";
import { AllocationMetrics, UnclassifiedBanner } from "./Header";
import { MembersPanel } from "./MembersPanel";
import { TaxonomyDock } from "./TaxonomyDock";
import { UNCLASSIFIED, views, bucketAt, descend, levelShare, nodeSlot, type LevelRow } from "./model";

export function Allocation() {
  const { t, i18n } = useLingui();
  const date = today();
  const invalidate = useInvalidate();
  const [view, setView] = useState("map");
  const [taxonomyId, setTaxonomyId] = useState<string | null>(null);
  const [path, setPath] = useState<string[]>([]);
  const [editing, setEditing] = useState(false);

  const portfolio = usePortfolio();
  const taxonomies = useTaxonomies();
  const selected = taxonomyId ?? taxonomies.data?.[0]?.id ?? null;
  const taxonomy = taxonomies.data?.find((t) => t.id === selected) ?? null;

  const allocation = useAllocation("taxonomy", selected, date);

  const levelKey = path[path.length - 1] ?? null;
  const members = useAllocationMembers(selected, levelKey, date);
  // A null id keeps the query idle: the tree is only drawn by two of the views.
  const tree = useAllocationTree(view === "tree" || view === "rings" ? selected : null, date);

  const positions = usePositions(date);
  const securities = useSecurities();

  const targets = useTargets();
  const target = targets.data?.find((t) => t.taxonomy_id === selected) ?? null;
  const plan = useRebalance(view === "target" ? (target?.id ?? null) : null, date);

  const currency = portfolio.data?.base_currency ?? "";
  const buckets = allocation.data?.buckets ?? [];
  const children = descend(buckets, path);
  const here = bucketAt(buckets, path);
  const unclassified = buckets.find((b) => b.key === UNCLASSIFIED);
  const atPositions = path.length > 0 && children.length === 0;
  const rows: LevelRow[] = atPositions
    ? // Excluded subjects are omitted from allocation charts but remain editable below.
      (members.data ?? [])
        .filter((member) => !member.excluded)
        .map((member, i) => ({
          kind: "position" as const,
          subjectKind: member.kind,
          key: member.subject_id,
          label: member.symbol,
          sub: member.name,
          value: member.value_base,
          // Core returns the weighted value; do not divide money in the UI.
          weight: member.weight,
          slot: slotFor(i),
        }))
    : children.map((bucket, i) => ({
        kind: "node" as const,
        key: bucket.key,
        label: bucketLabel(i18n, bucket),
        value: bucket.value_base,
        weight: levelShare(bucket.weight, here),
        slot: nodeSlot(taxonomy, bucket, i),
      }));

  const select = (id: string) => {
    setTaxonomyId(id);
    setPath([]);
  };

  const dialogs = useAllocationDialogs({
    taxonomy,
    target,
    levelKey,
    members: members.data,
    // A tree, the weights on it and the plan built from them always move together.
    onChanged: () => invalidate(...affects.taxonomies, ...affects.targets),
    onSelect: select,
    onDeleted: (id) => {
      // Avoid leaving a deleted taxonomy selected.
      if (id === selected) {
        setTaxonomyId(null);
        setPath([]);
      }
    },
    onNodeDeleted: (id) => setPath(path.filter((key) => key !== id)),
  });

  const enter = (key: string) => {
    const row = rows.find((r) => r.key === key);
    if (!row) return;
    // Cash has no security card; open its allocation editor instead.
    if (row.kind === "position" && row.subjectKind === "CASH") dialogs.openAssign(row.key);
    else if (row.kind === "position") dialogs.openCard(row.key);
    else setPath([...path, key]);
  };

  const enterTile = (key: string) => {
    if (key === UNCLASSIFIED || taxonomy?.nodes.some((n) => n.id === key)) setPath([...path, key]);
    else if (key.startsWith("cash:")) dialogs.openAssign(key);
    else dialogs.openCard(key);
  };

  if (taxonomies.isError) return <QueryError error={taxonomies.error} />;
  if (taxonomies.isPending) return <Pending />;

  return (
    <Page
      archetype="analysis"
      title={t`Allocation`}
      asOf={formatDay(date)}
      controls={
        <>
          <Seg label={t`View`} value={view} onChange={setView} options={views(i18n)} />

          <button
            type="button"
            className={`iconbtn${editing ? " iconbtn--on" : ""}`}
            onClick={() => setEditing(!editing)}
          >
            {editing ? (
              <>
                <CheckIcon /> <Trans>Done</Trans>
              </>
            ) : (
              <>
                <TreeStructureIcon /> <Trans>Edit tree</Trans>
              </>
            )}
          </button>
        </>
      }
      banner={
        taxonomy && unclassified ? (
          <UnclassifiedBanner
            taxonomyName={taxonomy.name}
            unclassified={unclassified}
            currency={currency}
            onShow={() => setPath([UNCLASSIFIED])}
          />
        ) : undefined
      }
      metrics={
        <AllocationMetrics
          allocation={allocation.data}
          here={here}
          parts={rows.filter((r) => r.key !== UNCLASSIFIED)}
          unclassified={unclassified}
          currency={currency}
          taxonomyName={taxonomy?.name}
          atPositions={atPositions}
        />
      }
    >
      {!taxonomy ? (
        <Empty
          title={t`No classification has been created`}
          action={
            <>
              <button type="button" className="btn" onClick={() => dialogs.openTaxonomy(null)}>
                <Trans>Create a classification</Trans>
              </button>
              <button
                type="button"
                className="btn btn--ghost"
                disabled={dialogs.busy !== null}
                onClick={() => dialogs.pickImport(null)}
              >
                <Trans>Import from CSV…</Trans>
              </button>
            </>
          }
        >
          <Trans>
            A classification is a tree of categories and the weights of instruments in them. A global fund is
            60 % United States and 40 % the rest of the world, which is why it is shares, not one category per
            instrument.
          </Trans>
        </Empty>
      ) : (
        <Async query={allocation}>
          {(data) => (
            <>
              <BreakdownPanel
                taxonomy={taxonomy}
                view={view}
                rows={rows}
                buckets={data.buckets}
                total={data.total_base}
                currency={currency}
                path={path}
                onPath={setPath}
                atPositions={atPositions}
                editing={editing}
                target={target}
                members={members}
                tree={tree}
                plan={plan}
                nameOf={(key) => securities.data?.find((sec) => sec.id === key)?.name}
                onEnter={enter}
                onEnterTile={enterTile}
                onAddNode={() => dialogs.openNode(null)}
                onEditNode={dialogs.openNode}
              />

              <MembersPanel
                taxonomy={taxonomy}
                members={members}
                levelKey={levelKey}
                currency={currency}
                onOpenCard={dialogs.openCard}
                onAssign={dialogs.openAssign}
              />
            </>
          )}
        </Async>
      )}

      <TaxonomyDock
        taxonomies={taxonomies.data}
        selected={selected}
        editing={editing}
        securityIds={positions.data?.rows.map((r) => r.security_id) ?? []}
        onSelect={select}
        onCreate={() => dialogs.openTaxonomy(null)}
        onEdit={dialogs.openTaxonomy}
        onImport={dialogs.pickImport}
        onGroup={dialogs.openGroup}
        onExport={dialogs.exportCsv}
        onDelete={dialogs.openDelete}
      />

      {dialogs.node}
    </Page>
  );
}
