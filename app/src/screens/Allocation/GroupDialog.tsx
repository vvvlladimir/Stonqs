import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Async, DataTable, Field, FormDialog, Metric, Metrics, Scrolly } from "../../components/ui";
import { useAttributeDefs, useTaxonomyGrouping } from "../../lib/queries";
import type { TaxonomyData, TaxonomyPreview } from "../../lib/types";

/**
 * Builds categories out of one attribute: a node per distinct value, every instrument carrying
 * that value assigned whole. What the tree already classifies is left alone — a split typed by
 * hand outranks a value read off a column.
 */
export function GroupDialog({
  into,
  busy,
  onClose,
  onGroup,
}: {
  into: TaxonomyData;
  busy: boolean;
  onClose: () => void;
  onGroup: (attributeId: string) => void;
}) {
  const { t } = useLingui();
  const defs = useAttributeDefs();
  // Only text groups: a node per distinct number or date would be a list, not a classification.
  const usable = (defs.data ?? []).filter((def) => def.kind === "TEXT");
  const [attributeId, setAttributeId] = useState<string | null>(null);
  const chosen = usable.find((def) => def.id === attributeId) ?? usable[0] ?? null;
  const plan = useTaxonomyGrouping(chosen?.id ?? null, into.id);

  return (
    <FormDialog
      title={t`Group "${into.name}" by an attribute`}
      onClose={onClose}
      onSubmit={() => chosen && onGroup(chosen.id)}
      busy={busy}
      busyLabel={t`Grouping…`}
      submitLabel={t`Group`}
      ready={chosen !== null && (plan.data?.nodes.length ?? 0) > 0}
      wide
    >
      {usable.length === 0 ? (
        <p className="panel__note">
          <Trans>
            No text attribute has been defined yet. Settings → Attributes is where one is created — a country,
            a sector, a strategy — and each instrument's form is where it is filled in.
          </Trans>
        </p>
      ) : (
        <>
          <Field
            label={t`Attribute`}
            hint={t`Categories are created from the values this attribute holds.`}
            options={usable.map((def) => ({ value: def.id, label: def.name }))}
            value={chosen?.id ?? ""}
            onChange={setAttributeId}
          />

          <Async query={plan}>{(data) => <Plan plan={data} />}</Async>
        </>
      )}
    </FormDialog>
  );
}

function Plan({ plan }: { plan: TaxonomyPreview }) {
  const { t } = useLingui();
  const assigned = plan.assignments.filter((a) => a.security_id !== null);
  const kept = plan.assignments.filter((a) => a.matched_by === "already_classified");
  const sizes = new Map<string, number>();
  for (const a of assigned) sizes.set(a.path[0], (sizes.get(a.path[0]) ?? 0) + 1);

  return (
    <>
      <Metrics>
        <Metric label={t`Categories`} value={String(plan.nodes.length)} hint={t`one per distinct value`} />
        <Metric
          label={t`Instruments`}
          value={String(assigned.length)}
          hint={assigned.length > 0 ? t`each lands in its category whole` : t`nothing to place`}
        />
        <Metric
          label={t`Left alone`}
          value={String(kept.length)}
          hint={kept.length > 0 ? t`already classified by hand` : t`nothing is classified yet`}
        />
      </Metrics>

      {plan.nodes.length > 0 && (
        <Scrolly max={200}>
          <DataTable
            card={false}
            rows={plan.nodes}
            rowKey={(node) => node.path[0]}
            columns={[
              { key: "name", align: "left", className: "nm", cell: (node) => node.path[0] },
              { key: "count", cell: (node) => sizes.get(node.path[0]) ?? 0 },
            ]}
          />
        </Scrolly>
      )}

      <p className="panel__note">
        <Trans>
          A category that already exists under the same name is reused, not duplicated, so grouping again
          after filling in more instruments adds only what is new.
        </Trans>
      </p>
    </>
  );
}
