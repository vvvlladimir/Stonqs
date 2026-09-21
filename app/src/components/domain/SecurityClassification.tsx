import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { PencilSimpleIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { affects, useAttributeDefs, useInvalidate, useTaxonomies } from "../../lib/queries";
import { Field, Form, Percent, Skeleton, SkeletonRows, Swatch } from "../ui";
import { formatPercent, signOf } from "../../lib/format";
import { slotFor } from "../../lib/plot";
import type { SecurityRow, TaxonomyData } from "../../lib/types";

/**
 * What kind of thing this instrument is, in one place: the trees it is split across and the
 * user's own attributes, both editable here. The figures beside it are the position's and
 * belong to the facts panel; these belong to the instrument and are the same in every screen.
 */
export function ClassificationPanel({
  securityId,
  security,
  onSplit,
}: {
  securityId: string;
  /** Absent when the position has no directory record; then there is nothing to edit. */
  security?: SecurityRow;
  onSplit: (taxonomy: TaxonomyData) => void;
}) {
  const { t } = useLingui();
  const taxonomies = useTaxonomies();

  if (taxonomies.isPending) return <SkeletonRows rows={4} h="1.6rem" />;

  return (
    <div className="stack">
      {taxonomies.data && taxonomies.data.length > 0 ? (
        <div className="stack secpop__tax">
          {taxonomies.data.map((taxonomy) => (
            <div key={taxonomy.id}>
              <div className="panel__note inline">
                <span>{taxonomy.name}</span>
                <span className="spacer" />
                <button
                  type="button"
                  className="iconbtn iconbtn--sm"
                  aria-label={t`Change the split in ${taxonomy.name}`}
                  onClick={() => onSplit(taxonomy)}
                >
                  <PencilSimpleIcon />
                </button>
              </div>
              <div className="chips">{pillsOf(taxonomy, securityId)}</div>
            </div>
          ))}
        </div>
      ) : (
        <p className="muted">
          <Trans>No classifications.</Trans>
        </p>
      )}

      {security && <AttributeForm security={security} />}
    </div>
  );
}

/** The instrument's own facts, saved through the directory command the securities screen uses,
 * so one write path keeps note, WKN and attributes consistent wherever they are edited. */
function AttributeForm({ security }: { security: SecurityRow }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const defs = useAttributeDefs();
  const [values, setValues] = useState<Record<string, string>>(security.attributes);
  const [note, setNote] = useState(security.note ?? "");

  const save = useMutation({
    mutationFn: () =>
      api.securitySave({
        id: security.id,
        symbol: security.symbol,
        name: security.name,
        currency: security.currency,
        kind: security.kind,
        isin: security.isin,
        data_source: security.data_source,
        data_symbol: security.data_symbol,
        quantity_step: security.quantity_step,
        wkn: security.wkn,
        note: note.trim() || null,
        attributes: values,
      }),
    onSuccess: () => invalidate(...affects.securities),
  });

  // Compare against what is stored so the button appears only once something really changed.
  const dirty =
    note !== (security.note ?? "") ||
    (defs.data ?? []).some((def) => (values[def.id] ?? "") !== (security.attributes[def.id] ?? ""));

  if (defs.isPending)
    return (
      <div className="stack">
        <Skeleton h="2.75rem" />
        <Skeleton h="4rem" />
      </div>
    );

  return (
    <Form
      onSubmit={() => save.mutate()}
      busy={save.isPending}
      error={save.error}
      ready={dirty}
      onCancel={
        dirty
          ? () => {
              setValues(security.attributes);
              setNote(security.note ?? "");
            }
          : undefined
      }
    >
      {(defs.data ?? []).map((def) => (
        <Field key={def.id} label={def.name} hint={def.unit ?? undefined}>
          <input
            type={def.kind === "DATE" ? "date" : "text"}
            inputMode={def.kind === "NUMBER" ? "decimal" : undefined}
            value={values[def.id] ?? ""}
            onChange={(e) => setValues({ ...values, [def.id]: e.target.value })}
          />
        </Field>
      ))}

      <Field label={t`Note`} hint={t`Why this instrument is held; nothing overwrites it`}>
        <textarea rows={2} value={note} onChange={(e) => setNote(e.target.value)} />
      </Field>
    </Form>
  );
}

/** Lists taxonomy assignments and hides redundant 100% weights. */
function pillsOf(taxonomy: TaxonomyData, securityId: string) {
  const parts = taxonomy.classifications.filter((c) => c.subject_id === securityId);
  if (parts.length === 0)
    return (
      <span className="pill pill--empty">
        <Trans>unclassified</Trans>
      </span>
    );

  return parts.map((part) => {
    const node = taxonomy.nodes.find((n) => n.id === part.node_id);
    const slot = taxonomy.nodes.findIndex((n) => n.id === part.node_id);
    const full = signOf(part.weight) !== "zero" && formatPercent(part.weight, { digits: 0 }) === "100 %";
    return (
      <span className="pill" key={part.node_id}>
        <Swatch slot={slotFor(slot)} />
        {node?.name ?? part.node_id}
        {!full && <Percent value={part.weight} digits={2} dim />}
      </span>
    );
  });
}
