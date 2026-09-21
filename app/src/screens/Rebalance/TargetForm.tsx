import { Fragment, useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { useMutation } from "@tanstack/react-query";
import { CaretRightIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { ErrorText, Field, FormDialog, PercentInput, Scrolly, Swatch } from "../../components/ui";
import { percentToWeight, slotOf } from "../../lib/taxonomy";
import type { TaxonomyData, TaxonomyNode } from "../../lib/types";

/** Target weights are entered relative to each parent node. */
export function TargetForm({
  taxonomies,
  onClose,
  onDone,
}: {
  taxonomies: TaxonomyData[];
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useLingui();
  const [name, setName] = useState("");
  const [taxonomyId, setTaxonomyId] = useState(taxonomies[0]?.id ?? "");
  const [weights, setWeights] = useState<Record<string, string>>({});
  const [open, setOpen] = useState<Record<string, boolean>>({});

  const save = useMutation({ mutationFn: api.targetSave, onSuccess: onDone });
  const taxonomy = taxonomies.find((t) => t.id === taxonomyId);
  const nodes = taxonomy?.nodes ?? [];
  const roots = nodes.filter((n) => n.parent_id === null);
  const childrenOf = (id: string) => nodes.filter((n) => n.parent_id === id);
  const percentOf = (id: string) => Number((weights[id] ?? "").replace(",", ".") || "0");

  // Validate each sibling group separately; percentages are parent-relative.
  const overflow = (siblings: TaxonomyNode[]) =>
    siblings.reduce((total, node) => total + percentOf(node.id), 0) > 100.0001;
  const broken = [roots, ...nodes.map((n) => childrenOf(n.id))].some(
    (siblings) => siblings.length > 0 && overflow(siblings),
  );

  const submit = () => {
    save.mutate({
      id: null,
      taxonomy_id: taxonomyId,
      name,
      weights: Object.entries(weights)
        .filter(([, value]) => value.trim() !== "")
        .map(([node, value]) => [node, percentToWeight(value)] as [string, string]),
    });
  };

  const rows = (siblings: TaxonomyNode[], depth: number) => (
    <div className={`colpick${depth > 1 ? " colpick--nest" : ""}`}>
      {siblings.map((node) => {
        const kids = childrenOf(node.id);
        return (
          <Fragment key={node.id}>
            <label>
              <span>{node.name}</span>
              <PercentInput
                value={weights[node.id] ?? ""}
                onChange={(value) => setWeights({ ...weights, [node.id]: value })}
              />
            </label>
            {kids.length > 0 && rows(kids, depth + 1)}
          </Fragment>
        );
      })}
    </div>
  );

  return (
    <FormDialog
      title={t`New target`}
      onClose={onClose}
      onSubmit={submit}
      busy={save.isPending}
      error={save.error}
      ready={!broken && name.trim() !== ""}
      wide
    >
      <Field label={t`Target name`}>
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder="60 / 40" />
      </Field>

      <Field
        label={t`Taxonomy`}
        options={taxonomies.map((tree) => ({ value: tree.id, label: tree.name }))}
        value={taxonomyId}
        onChange={setTaxonomyId}
      />

      {roots.length === 0 ? (
        <p className="muted">
          <Trans>The tree has no categories — there is nowhere to put a weight.</Trans>
        </p>
      ) : (
        <>
          <Scrolly className="acc" max={380}>
            {roots.map((root) => {
              const kids = childrenOf(root.id);
              const inside = kids.reduce((total, kid) => total + percentOf(kid.id), 0);
              const expanded = open[root.id] ?? inside > 0;
              return (
                <div className="acc__group" key={root.id}>
                  <div className="acc__head acc__head--field">
                    {kids.length > 0 ? (
                      <button
                        type="button"
                        className="acc__toggle"
                        aria-expanded={expanded}
                        aria-label={expanded ? t`Collapse` : t`Expand`}
                        onClick={() => setOpen({ ...open, [root.id]: !expanded })}
                      >
                        <CaretRightIcon className={`acc__caret${expanded ? " acc__caret--on" : ""}`} />
                      </button>
                    ) : (
                      <span />
                    )}
                    <Swatch slot={slotOf(taxonomy!, root)} />
                    <span className="acc__name">{root.name}</span>
                    <span className="acc__meta num">
                      {kids.length === 0
                        ? ""
                        : inside > 0
                          ? t`${Math.round(inside)} % inside`
                          : `${kids.length}`}
                    </span>
                    <PercentInput
                      label={t`Weight of "${root.name}", %`}
                      value={weights[root.id] ?? ""}
                      onChange={(value) => setWeights({ ...weights, [root.id]: value })}
                    />
                  </div>
                  {expanded && kids.length > 0 && rows(kids, 1)}
                </div>
              );
            })}
          </Scrolly>

          {broken ? (
            <ErrorText>
              <Trans>
                Siblings in one category add up to more than a hundred percent — that cannot be saved.
              </Trans>
            </ErrorText>
          ) : (
            <p className="panel__note">
              <Trans>
                Percentages are a share inside the parent: "equities 42 %, of which the core is 88 %". A
                category left empty stays outside the target.
              </Trans>
            </p>
          )}
        </>
      )}
    </FormDialog>
  );
}
