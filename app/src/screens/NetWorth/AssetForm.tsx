import { useLingui } from "@lingui/react/macro";
import { Field, FieldPair, FormDialog } from "../../components/ui";
import { OWED_ASSET_KINDS, OWNED_ASSET_KINDS, assetKindLabel } from "../../lib/kinds";
import type { Asset, AssetInput, AssetKind } from "../../lib/types";
import { isOwed } from "./model";

/** One form for both sides: a debt adds a schedule and the thing it is secured by. */
export function AssetForm({
  draft,
  assets,
  onChange,
  onSubmit,
  onClose,
  busy,
  error,
}: {
  draft: AssetInput;
  assets: Asset[];
  onChange: (draft: AssetInput) => void;
  onSubmit: () => void;
  onClose: () => void;
  busy: boolean;
  error: Error | null;
}) {
  const { t, i18n } = useLingui();
  const owed = isOwed(draft);
  const kinds = owed ? OWED_ASSET_KINDS : OWNED_ASSET_KINDS;
  // Only things owned can secure a debt, and a debt never secures itself.
  const securable = assets.filter((a) => !OWED_ASSET_KINDS.includes(a.kind) && a.id !== draft.id);

  const title = draft.id ? (owed ? t`Edit the debt` : t`Edit the asset`) : owed ? t`New debt` : t`New asset`;

  return (
    <FormDialog
      title={title}
      onClose={onClose}
      onSubmit={onSubmit}
      busy={busy}
      error={error}
      ready={draft.name.trim() !== "" && draft.currency.trim() !== ""}
    >
      <Field label={t`Name`}>
        <input value={draft.name} autoFocus onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>
      <FieldPair>
        <Field
          label={t`Kind`}
          value={draft.kind}
          options={kinds.map((kind) => ({ value: kind, label: assetKindLabel(i18n, kind) }))}
          onChange={(kind: AssetKind) => onChange({ ...draft, kind })}
        />
        <Field label={t`Currency`}>
          <input
            value={draft.currency}
            onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
          />
        </Field>
      </FieldPair>

      {draft.id === null && (
        <FieldPair>
          <Field
            label={owed ? t`Still owed` : t`Value`}
            hint={t`A figure you write yourself; nothing here is priced by a market.`}
          >
            <input
              inputMode="decimal"
              value={draft.amount ?? ""}
              onChange={(e) => onChange({ ...draft, amount: e.target.value })}
            />
          </Field>
          <Field label={t`Valued on`} hint={t`The day this figure is from.`}>
            <input
              type="date"
              value={draft.valued_on ?? ""}
              onChange={(e) => onChange({ ...draft, valued_on: e.target.value || null })}
            />
          </Field>
        </FieldPair>
      )}

      {owed && (
        <>
          <Field
            label={t`Secured by`}
            hint={t`Shows the relationship — it moves no figure and records no payment.`}
            value={draft.secured_by ?? ""}
            placeholder={t`Nothing`}
            options={securable.map((asset) => ({ value: asset.id, label: asset.name }))}
            onChange={(id) => onChange({ ...draft, secured_by: id || null })}
          />
          <FieldPair>
            <Field
              label={t`Interest rate, % a year`}
              hint={t`Describes the debt ahead; what is owed today stays the figure you wrote.`}
            >
              <input
                inputMode="decimal"
                value={draft.rate ?? ""}
                onChange={(e) => onChange({ ...draft, rate: e.target.value || null })}
              />
            </Field>
            <Field label={t`Monthly payment`}>
              <input
                inputMode="decimal"
                value={draft.monthly_payment ?? ""}
                onChange={(e) => onChange({ ...draft, monthly_payment: e.target.value || null })}
              />
            </Field>
          </FieldPair>
          <Field label={t`Ends on`} hint={t`When the debt is meant to be gone. Optional.`}>
            <input
              type="date"
              value={draft.ends_on ?? ""}
              onChange={(e) => onChange({ ...draft, ends_on: e.target.value || null })}
            />
          </Field>
        </>
      )}

      <Field
        label={owed ? t`Repaid on` : t`Closed on`}
        hint={t`From this day it is out of net worth, and its history stays.`}
      >
        <input
          type="date"
          value={draft.closed_on ?? ""}
          onChange={(e) => onChange({ ...draft, closed_on: e.target.value || null })}
        />
      </Field>
      <Field label={t`Note`}>
        <input
          value={draft.note ?? ""}
          onChange={(e) => onChange({ ...draft, note: e.target.value || null })}
        />
      </Field>
    </FormDialog>
  );
}
