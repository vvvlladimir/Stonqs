import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Field, FormDialog, Modal } from "../../components/ui";
import { useAttributeDefs } from "../../lib/queries";
import type { TaxonomyData } from "../../lib/types";

/**
 * A new tree is a name and, if the user has an attribute to build it from, that attribute:
 * "Country" is created because a Country attribute is already filled in. Filling an existing
 * tree is the tab menu's business — there the tree is what may not be overwritten.
 */
export function TaxonomyDialog({
  taxonomy,
  busy,
  onClose,
  onSave,
}: {
  taxonomy: TaxonomyData | null;
  busy: boolean;
  onClose: () => void;
  onSave: (name: string, attributeId: string | null) => void;
}) {
  const { t } = useLingui();
  const defs = useAttributeDefs();
  // Only text groups: a node per distinct number or date would be a list, not a classification.
  const usable = (defs.data ?? []).filter((def) => def.kind === "TEXT");
  const [name, setName] = useState(taxonomy?.name ?? "");
  const [attributeId, setAttributeId] = useState("");

  const pickAttribute = (id: string) => {
    setAttributeId(id);
    // The attribute's name is the name of the tree it builds, until the user says otherwise.
    const def = usable.find((d) => d.id === id);
    if (def && name.trim() === "") setName(def.name);
  };

  return (
    <FormDialog
      title={taxonomy ? t`Edit classification` : t`New classification`}
      onClose={onClose}
      onSubmit={() => onSave(name, attributeId || null)}
      busy={busy}
      busyLabel={t`Creating…`}
      submitLabel={taxonomy ? t`Save` : t`Create`}
      ready={name.trim() !== ""}
    >
      <Field label={t`Name`}>
        <input value={name} autoFocus onChange={(e) => setName(e.target.value)} />
      </Field>

      {!taxonomy && usable.length > 0 && (
        <Field
          label={t`Fill from attribute`}
          hint={t`A category per value the attribute holds; instruments without a value stay unclassified. The tree is yours afterwards — regrouping never overwrites a split you typed by hand.`}
          placeholder={t`Leave empty`}
          options={usable.map((def) => ({ value: def.id, label: def.name }))}
          value={attributeId}
          onChange={pickAttribute}
        />
      )}
    </FormDialog>
  );
}

/** Confirmation for a delete that also drops the tree's classifications and target. */
export function DeleteTaxonomyModal({
  taxonomy,
  busy,
  onClose,
  onConfirm,
}: {
  taxonomy: TaxonomyData;
  busy: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useLingui();
  return (
    <Modal
      title={t`Delete this classification?`}
      onClose={onClose}
      foot={
        <>
          <span className="spacer" />
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            <Trans>Cancel</Trans>
          </button>
          <button type="button" className="btn btn--danger" disabled={busy} onClick={onConfirm}>
            {busy ? t`Deleting…` : t`Delete`}
          </button>
        </>
      }
    >
      <p>
        <Trans>
          "{taxonomy.name}" disappears together with every category, every instrument split across them and
          the target that stood on them. This cannot be undone.
        </Trans>
      </p>
      <p className="panel__note">
        <Trans>The instruments' shares return to "unclassified"; other trees are untouched.</Trans>
      </p>
    </Modal>
  );
}
