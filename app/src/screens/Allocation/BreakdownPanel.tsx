import { bucketLabel } from "../../lib/taxonomy";
import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { ArrowUUpLeftIcon, PencilSimpleIcon, PlusIcon } from "@phosphor-icons/react";
import type { UseQueryResult } from "@tanstack/react-query";
import { Crumbs, Money, Panel, Percent, Tree, TreeNode, type Crumb } from "../../components/ui";
import type {
  Allocation,
  AllocationBucket,
  AllocationTarget,
  NodeMember,
  RebalancePlan,
  TaxonomyData,
  TaxonomyNode,
} from "../../lib/types";
import { BreakdownView } from "./BreakdownView";
import { UNCLASSIFIED, bucketAt, labelOf, targetOf, type LevelRow } from "./model";

/** Chart of the current level next to the list of its categories. */
export function BreakdownPanel({
  taxonomy,
  view,
  rows,
  buckets,
  total,
  currency,
  path,
  onPath,
  atPositions,
  editing,
  target,
  members,
  tree,
  plan,
  nameOf,
  onEnter,
  onEnterTile,
  onAddNode,
  onEditNode,
}: {
  taxonomy: TaxonomyData;
  view: string;
  rows: LevelRow[];
  buckets: AllocationBucket[];
  total: string;
  currency: string;
  path: string[];
  onPath: (path: string[]) => void;
  atPositions: boolean;
  editing: boolean;
  target: AllocationTarget | null;
  members: UseQueryResult<NodeMember[]>;
  tree: UseQueryResult<Allocation>;
  plan: UseQueryResult<RebalancePlan>;
  nameOf: (key: string) => string | undefined;
  onEnter: (key: string) => void;
  onEnterTile: (key: string) => void;
  onAddNode: () => void;
  onEditNode: (node: TaxonomyNode) => void;
}) {
  const { t, i18n } = useLingui();
  const here = bucketAt(buckets, path);
  const parts = rows.filter((r) => r.key !== UNCLASSIFIED);
  const crumbs: Crumb[] = [
    { id: "", label: taxonomy.name },
    ...path.map((key, i) => ({ id: key, label: labelOf(i18n, buckets, path.slice(0, i + 1)) })),
  ];

  return (
    <Panel
      title={t`Breakdown`}
      note={
        view === "tree" || view === "rings"
          ? path.length === 0
            ? view === "rings"
              ? t`the whole portfolio down to instruments, a ring is a level`
              : t`the whole portfolio down to instruments, area is share`
            : view === "rings"
              ? t`this branch down to instruments, a ring is a level`
              : t`this branch down to instruments, area is share`
          : atPositions
            ? plural(rows.length, {
                one: "# instrument on this level",
                other: "# instruments on this level",
              })
            : plural(parts.length, {
                one: "# category on this level",
                other: "# categories on this level",
              })
      }
    >
      <div className="inline">
        <Crumbs
          items={crumbs}
          onPick={(id) => onPath(id === "" ? [] : path.slice(0, path.indexOf(id) + 1))}
        />
        {path.length > 0 && (
          <>
            <span className="spacer" />
            <button type="button" className="crumb" onClick={() => onPath(path.slice(0, -1))}>
              <ArrowUUpLeftIcon /> <Trans>Back</Trans>
            </button>
          </>
        )}
      </div>

      <div className="bd">
        <div>
          <BreakdownView
            view={view}
            taxonomy={taxonomy}
            rows={rows}
            currency={currency}
            path={path}
            onPath={onPath}
            atPositions={atPositions}
            target={target}
            members={members}
            tree={tree}
            plan={plan}
            nameOf={nameOf}
            onEnter={onEnter}
            onEnterTile={onEnterTile}
          />
        </div>

        <aside className={`bd__side${editing ? " is-editing-tax" : ""}`}>
          <div className="panel__head">
            <h3>{here ? bucketLabel(i18n, here) : taxonomy.name}</h3>
            <span className="spacer" />
            {editing && (
              <button type="button" className="wbtn" aria-label={t`Add category`} onClick={onAddNode}>
                <PlusIcon />
              </button>
            )}
          </div>

          <Tree>
            {rows.map((row) => {
              const node = taxonomy.nodes.find((n) => n.id === row.key);
              const share = targetOf(target, row.key);
              return (
                <TreeNode
                  key={row.key}
                  slot={row.slot}
                  unassigned={row.key === UNCLASSIFIED}
                  name={
                    <>
                      {row.label}
                      {share && (
                        <span className="tag">
                          <Trans>
                            target <Percent value={share} digits={0} />
                          </Trans>
                        </span>
                      )}
                      {row.kind === "node" && <span className="dim"> ›</span>}
                    </>
                  }
                  value={<Money value={row.value} currency={currency} digits={0} />}
                  weight={<Percent value={row.weight} digits={1} />}
                  tools={
                    node && (
                      <button
                        type="button"
                        className="wbtn"
                        aria-label={t`Edit category`}
                        onClick={(e) => {
                          e.stopPropagation();
                          onEditNode(node);
                        }}
                      >
                        <PencilSimpleIcon />
                      </button>
                    )
                  }
                  onClick={() => onEnter(row.key)}
                />
              );
            })}
          </Tree>

          <div className="bd__total">
            <span className="muted">
              <Trans>Total</Trans>
            </span>
            <span className="spacer" />
            <Money value={here ? here.value_base : total} currency={currency} className="nm" />
          </div>
        </aside>
      </div>
    </Panel>
  );
}
