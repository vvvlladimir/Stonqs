import { useLingui } from "@lingui/react/macro";
import { Field, FieldSet, FormDialog } from "../../components/ui";
import { useAttributeDefs, useQuoteProviders } from "../../lib/queries";
import { SECURITY_KINDS, securityKindLabel } from "../../lib/kinds";
import type { SecurityAttributeDef, SecurityInput } from "../../lib/types";

export function SecurityForm({
  draft,
  onChange,
  onSubmit,
  onCancel,
  pending,
  error,
}: {
  draft: SecurityInput;
  onChange: (draft: SecurityInput) => void;
  onSubmit: () => void;
  onCancel: () => void;
  pending: boolean;
  error: Error | null;
}) {
  const { t, i18n } = useLingui();
  return (
    <FormDialog
      title={draft.id ? t`Edit instrument` : t`New instrument`}
      onClose={onCancel}
      onSubmit={onSubmit}
      busy={pending}
      error={error}
    >
      <Field label={t`Ticker`}>
        <input
          value={draft.symbol}
          onChange={(e) => onChange({ ...draft, symbol: e.target.value.toUpperCase() })}
        />
      </Field>

      <Field label={t`Name`}>
        <input value={draft.name} onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>

      <Field label={t`Currency`} hint={t`The instrument's trading and quote currency`}>
        <input
          value={draft.currency}
          maxLength={3}
          onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
        />
      </Field>

      <Field
        label={t`Kind`}
        options={SECURITY_KINDS.map((kind) => ({ value: kind, label: securityKindLabel(i18n, kind) }))}
        value={draft.kind}
        onChange={(kind) => onChange({ ...draft, kind })}
      />

      <Field label="ISIN">
        <input
          value={draft.isin ?? ""}
          onChange={(e) => onChange({ ...draft, isin: e.target.value || null })}
        />
      </Field>

      <Field label="WKN" hint={t`The German securities number, when the broker prints one`}>
        <input
          value={draft.wkn ?? ""}
          onChange={(e) => onChange({ ...draft, wkn: e.target.value.toUpperCase() || null })}
        />
      </Field>

      <ProviderField draft={draft} onChange={onChange} />

      <Field label={t`Provider symbol`} hint={t`When it differs from the ticker`}>
        <input
          value={draft.data_symbol ?? ""}
          onChange={(e) => onChange({ ...draft, data_symbol: e.target.value || null })}
        />
      </Field>

      <FallbackSymbols draft={draft} onChange={onChange} />

      <Field
        label={t`Quantity step`}
        hint={t`A property of the broker, not the instrument: one allows fractions, another whole lots. Empty means take it from trade quantities, and from the instrument kind when there are none`}
      >
        <input
          inputMode="decimal"
          value={draft.quantity_step ?? ""}
          onChange={(e) => onChange({ ...draft, quantity_step: e.target.value || null })}
        />
      </Field>

      <Field label={t`Note`} hint={t`Why this instrument is held; nothing overwrites it`}>
        <textarea
          rows={3}
          value={draft.note ?? ""}
          onChange={(e) => onChange({ ...draft, note: e.target.value || null })}
        />
      </Field>

      <AttributeFields draft={draft} onChange={onChange} />
    </FormDialog>
  );
}

/** The attributes the user defined, in the order they defined them. Values are keyed by
 * attribute id, so renaming one keeps what was filled in. */
function AttributeFields({
  draft,
  onChange,
}: {
  draft: SecurityInput;
  onChange: (d: SecurityInput) => void;
}) {
  const { t } = useLingui();
  const defs = useAttributeDefs();
  if (!defs.data || defs.data.length === 0) return null;

  const values = draft.attributes ?? {};
  const set = (def: SecurityAttributeDef, value: string) =>
    onChange({ ...draft, attributes: { ...values, [def.id]: value } });

  return (
    <FieldSet label={t`Attributes`}>
      {defs.data.map((def) => (
        <Field key={def.id} label={def.name} hint={def.unit ?? undefined}>
          <input
            type={def.kind === "DATE" ? "date" : "text"}
            inputMode={def.kind === "NUMBER" ? "decimal" : undefined}
            value={values[def.id] ?? ""}
            onChange={(e) => set(def, e.target.value)}
          />
        </Field>
      ))}
    </FieldSet>
  );
}

/** Choose only from quote sources registered by the host. */
function ProviderField({ draft, onChange }: { draft: SecurityInput; onChange: (d: SecurityInput) => void }) {
  const { t } = useLingui();
  const providers = useQuoteProviders();
  const known = providers.data ?? [];
  // Do not silently replace a removed provider with manual quotes.
  const options =
    draft.data_source && !known.includes(draft.data_source) ? [...known, draft.data_source] : known;

  return (
    <Field
      label={t`Quote provider`}
      hint={t`Empty means prices are entered by hand`}
      placeholder={t`— manual —`}
      options={options.map((id) => ({ value: id, label: id }))}
      value={draft.data_source}
      onChange={(id) => onChange({ ...draft, data_source: id || null })}
    />
  );
}

/** The instrument's symbol at every other quote source: a source is asked in the own one's place
 * only when it knows the instrument, and each spells the ticker its own way. */
function FallbackSymbols({
  draft,
  onChange,
}: {
  draft: SecurityInput;
  onChange: (d: SecurityInput) => void;
}) {
  const { t } = useLingui();
  const providers = useQuoteProviders();
  const others = (providers.data ?? []).filter((id) => id !== draft.data_source);
  if (!draft.data_source || others.length === 0) return null;
  const symbols = draft.other_symbols ?? {};

  return (
    <FieldSet label={t`Fallback symbols`}>
      {others.map((id) => (
        <Field key={id} label={id} hint={t`Asked only when the own source fails`}>
          <input
            value={symbols[id] ?? ""}
            onChange={(e) => onChange({ ...draft, other_symbols: { ...symbols, [id]: e.target.value } })}
          />
        </Field>
      ))}
    </FieldSet>
  );
}
