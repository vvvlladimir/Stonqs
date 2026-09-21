import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { ErrorText, Field, FormDialog, Percent } from "../../components/ui";
import { percentToWeight, weightToPercent } from "../../lib/taxonomy";
import { slotVar } from "../../lib/plot";
import type { AllocationTarget, TaxonomyData, TaxonomyNode } from "../../lib/types";
import { SLOTS, ancestorFactor, targetOf } from "./model";

export function NodeDialog({
  taxonomy,
  node,
  parentId,
  target,
  onClose,
  onSaved,
  onDeleted,
}: {
  taxonomy: TaxonomyData;
  node: TaxonomyNode | null;
  parentId: string | null;
  target: AllocationTarget | null;
  onClose: () => void;
  onSaved: () => void;
  onDeleted: (id: string) => void;
}) {
  const { t, i18n } = useLingui();
  const [name, setName] = useState(node?.name ?? "");
  const [color, setColor] = useState<number | null>(node?.color ?? null);
  const [share, setShare] = useState(node ? weightToPercent(targetOf(target, node.id) ?? undefined) : "");
  const parent = taxonomy.nodes.find((n) => n.id === (node ? node.parent_id : parentId)) ?? null;
  const factor = ancestorFactor(taxonomy, target, parent);
  const typed = share.trim() === "" ? null : Number(percentToWeight(share));
  const whole = typed !== null && factor !== 1 ? typed * factor : null;

  const save = useMutation({
    mutationFn: async () => {
      const saved = await api.taxonomyNodeSave({
        id: node?.id ?? null,
        taxonomy_id: taxonomy.id,
        parent_id: node ? node.parent_id : parentId,
        name,
        rank: node?.rank ?? taxonomy.nodes.length,
        color,
      });
      await saveTargetWeight(i18n, taxonomy, target, nodeIdOf(saved, node), share);
    },
    onSuccess: onSaved,
  });

  const remove = useMutation({
    mutationFn: async () => {
      if (!node) return;
      await api.taxonomyNodeDelete(node.id);
      await saveTargetWeight(i18n, taxonomy, target, node.id, "");
    },
    onSuccess: () => node && onDeleted(node.id),
  });

  return (
    <FormDialog
      title={node ? t`Edit category` : t`New category`}
      onClose={onClose}
      onSubmit={() => save.mutate()}
      busy={save.isPending}
      error={save.error}
      ready={name.trim() !== ""}
      lead={
        node && (
          <button
            type="button"
            className="btn btn--danger"
            disabled={remove.isPending}
            onClick={() => {
              if (confirm(t`Delete this category? The instrument splits across it disappear too.`)) {
                remove.mutate();
              }
            }}
          >
            <Trans>Delete</Trans>
          </button>
        )
      }
    >
      <Field label={t`Name`}>
        <input value={name} autoFocus onChange={(e) => setName(e.target.value)} />
      </Field>
      <Field
        label={t`Colour`}
        hint={t`A palette slot, not an arbitrary colour: colours belong to the theme.`}
      >
        <div className="swatches">
          {SLOTS.map((slot) => (
            <button
              key={slot}
              type="button"
              aria-label={t`Slot ${slot}`}
              aria-pressed={color === slot}
              className="swatch"
              style={slotVar(slot)}
              onClick={() => setColor(color === slot ? null : slot)}
            />
          ))}
        </div>
      </Field>
      <Field
        label={parent ? t`Target weight, % inside "${parent.name}"` : t`Target weight, % of the portfolio`}
        hint={
          parent
            ? t`A share inside the parent: "equities 42 %, of which the core is 88 %". Empty means no target.`
            : t`Empty means no target. With one, the drift and the trades that close it appear.`
        }
      >
        <input inputMode="decimal" value={share} onChange={(e) => setShare(e.target.value)} />
        {whole !== null && (
          <p className="panel__note">
            <Trans>
              Of the whole portfolio that is <Percent value={String(whole)} digits={2} />.
            </Trans>
          </p>
        )}
      </Field>
      {parentId && !node && (
        <p className="panel__note">
          <Trans>
            The category is created inside "{taxonomy.nodes.find((n) => n.id === parentId)?.name}".
          </Trans>
        </p>
      )}
      <ErrorText error={remove.error} />
    </FormDialog>
  );
}

function nodeIdOf(saved: unknown, node: TaxonomyNode | null): string {
  if (node) return node.id;
  const id = (saved as { id?: string } | null)?.id;
  return id ?? "";
}

/** Save one target weight without rebuilding unrelated nodes. */
async function saveTargetWeight(
  i18n: I18n,
  taxonomy: TaxonomyData,
  target: AllocationTarget | null,
  nodeId: string,
  percent: string,
): Promise<void> {
  if (nodeId === "") return;
  const weight = percent.trim() === "" ? null : percentToWeight(percent);
  const kept = (target?.weights ?? [])
    .filter((w) => w.node_id !== nodeId)
    .map((w) => [w.node_id, w.weight] as [string, string]);
  const weights = weight ? [...kept, [nodeId, weight] as [string, string]] : kept;
  if (weights.length === 0 && !target) return;

  await api.targetSave({
    id: target?.id ?? null,
    taxonomy_id: taxonomy.id,
    name: target?.name ?? i18n._(msg`Target · ${taxonomy.name}`),
    weights,
  });
}
