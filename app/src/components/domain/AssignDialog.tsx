import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { ArrowUUpLeftIcon, CaretRightIcon, ProhibitIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { Banner, ErrorText, FormDialog, PercentInput, Scrolly, SearchBox, Swatch } from "../ui";
import { affects, useInvalidate } from "../../lib/queries";
import { branchName, percentToWeight, slotOf, weightToPercent } from "../../lib/taxonomy";
import type { TaxonomyData, TaxonomyNode } from "../../lib/types";

/** Edit a subject's percentage allocation across leaf nodes. */
export function AssignDialog({
  taxonomy,
  subjectId,
  name,
  onClose,
  onSaved,
}: {
  taxonomy: TaxonomyData;
  subjectId: string;
  name: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const excluded = taxonomy.excluded.includes(subjectId);
  const leaves = taxonomy.nodes.filter(
    (node) => !taxonomy.nodes.some((other) => other.parent_id === node.id),
  );
  const mine = new Map(
    taxonomy.classifications.filter((c) => c.subject_id === subjectId).map((c) => [c.node_id, c]),
  );
  const [draft, setDraft] = useState<Record<string, string>>(() =>
    Object.fromEntries(leaves.map((node) => [node.id, weightToPercent(mine.get(node.id)?.weight)])),
  );
  const [query, setQuery] = useState("");
  // Expand groups containing assignments; expanding all would hide the existing layout.
  const [open, setOpen] = useState<Record<string, boolean>>({});

  const save = useMutation({
    mutationFn: async () => {
      for (const node of leaves) {
        const percent = Number((draft[node.id] ?? "").replace(",", ".") || "0");
        if (percent > 0) {
          await api.classificationSave({
            subject_id: subjectId,
            node_id: node.id,
            weight: percentToWeight(draft[node.id]),
          });
        } else if (mine.has(node.id)) {
          await api.classificationDelete(subjectId, node.id);
        }
      }
    },
    // Invalidating here rather than in every caller: a split is read by the trees, the charts
    // and the card that opened this dialog.
    onSuccess: () => {
      invalidate(...affects.taxonomies);
      onSaved();
    },
  });

  // Exclusion is independent of draft weights; saved weights return when re-enabled.
  const toggle = useMutation({
    mutationFn: () => api.taxonomyExclude(taxonomy.id, subjectId, !excluded),
    onSuccess: () => invalidate(...affects.taxonomies),
  });

  const sum = leaves.reduce(
    (total, node) => total + Number((draft[node.id] ?? "").replace(",", ".") || "0"),
    0,
  );
  const over = sum > 100.0001;

  // Grouping keeps large taxonomies readable and opens only assigned groups.
  const percentOf = (id: string) => Number((draft[id] ?? "").replace(",", ".") || "0");
  const needle = query.trim().toLowerCase();
  const groups = new Map<string, { label: string; slot: number; nodes: TaxonomyNode[] }>();
  for (const node of leaves) {
    const parent = taxonomy.nodes.find((n) => n.id === node.parent_id);
    const label = parent ? branchName(taxonomy, parent) : t`No parent`;
    const key = parent?.id ?? "";
    if (needle && !`${label} ${node.name}`.toLowerCase().includes(needle)) continue;
    const group = groups.get(key) ?? {
      label,
      slot: slotOf(taxonomy, parent ?? node),
      nodes: [],
    };
    group.nodes.push(node);
    groups.set(key, group);
  }

  return (
    <FormDialog
      title={t`Split: ${name}`}
      onClose={onClose}
      onSubmit={() => save.mutate()}
      busy={save.isPending}
      error={save.error}
      ready={!over && !excluded}
      wide
      lead={
        <button
          type="button"
          className={excluded ? "btn btn--ghost" : "btn btn--danger"}
          disabled={toggle.isPending}
          onClick={() => toggle.mutate()}
        >
          {excluded ? (
            <>
              <ArrowUUpLeftIcon /> <Trans>Enable</Trans>
            </>
          ) : (
            <>
              <ProhibitIcon /> <Trans>Disable</Trans>
            </>
          )}
        </button>
      }
    >
      {excluded && (
        <Banner tone="info">
          <Trans>
            The subject is disabled in "{taxonomy.name}": it is absent from the chart, from the shares and
            from "unclassified". The split below is kept and returns to the maths as soon as you enable it.
          </Trans>
        </Banner>
      )}
      {leaves.length === 0 ? (
        <p className="muted">
          <Trans>The tree has no categories — there is nothing to split across.</Trans>
        </p>
      ) : (
        <>
          {leaves.length > 8 && (
            <SearchBox value={query} onChange={setQuery} placeholder={t`Find a category`} />
          )}

          <Scrolly className="acc" max={380}>
            {[...groups.entries()].map(([key, group]) => {
              const filled = group.nodes.filter((n) => percentOf(n.id) > 0);
              const total = group.nodes.reduce((t, n) => t + percentOf(n.id), 0);
              const expanded = open[key] ?? (needle !== "" || filled.length > 0);
              return (
                <div className="acc__group" key={key}>
                  <button
                    type="button"
                    className="acc__head"
                    aria-expanded={expanded}
                    onClick={() => setOpen({ ...open, [key]: !expanded })}
                  >
                    <CaretRightIcon className={`acc__caret${expanded ? " acc__caret--on" : ""}`} />
                    <Swatch slot={group.slot} />
                    <span className="acc__name">{group.label}</span>
                    <span className="acc__meta num">
                      {filled.length > 0 ? `${Math.round(total)} %` : `${group.nodes.length}`}
                    </span>
                  </button>
                  {expanded && (
                    <div className="colpick">
                      {group.nodes.map((node) => (
                        <label key={node.id}>
                          <span className="acc__leaf">{node.name}</span>
                          <PercentInput
                            value={draft[node.id] ?? ""}
                            disabled={excluded}
                            onChange={(value) => setDraft({ ...draft, [node.id]: value })}
                          />
                        </label>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
            {groups.size === 0 && (
              <p className="muted">
                <Trans>Nothing found.</Trans>
              </p>
            )}
          </Scrolly>
          {over ? (
            <ErrorText>
              <Trans>The sum is {Math.round(sum)} % — over a hundred, that cannot be saved</Trans>
            </ErrorText>
          ) : (
            <p className="panel__note">
              <Trans>
                Sum {Math.round(sum)} % · the remaining {Math.max(0, Math.round(100 - sum))} % goes to
                "unclassified"
              </Trans>
            </p>
          )}
        </>
      )}
      <ErrorText error={toggle.error} />
    </FormDialog>
  );
}
