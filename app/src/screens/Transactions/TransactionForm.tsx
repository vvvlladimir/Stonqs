import { useLingui } from "@lingui/react/macro";
import { Field, FieldPair, FormDialog } from "../../components/ui";
import { EDITABLE_TRANSACTION_KINDS, transactionLabel } from "../../lib/kinds";
import type { AccountKind, TransactionInput, TransactionKind } from "../../lib/types";
import { QUANTITY_KINDS, SECURITY_KINDS } from "./model";

interface FormProps {
  draft: TransactionInput;
  accounts: Array<{ id: string; name: string; currency: string; kind: AccountKind }>;
  securities: Array<{ id: string; symbol: string }>;
  onChange: (draft: TransactionInput) => void;
  onSubmit: () => void;
  onCancel: () => void;
  pending: boolean;
  error: Error | null;
}

/** A currency code as typed: upper case, and empty means the transaction's own. */
function code(value: string): string | null {
  return value.toUpperCase().trim() || null;
}

export function TransactionForm({
  draft,
  accounts,
  securities,
  onChange,
  onSubmit,
  onCancel,
  pending,
  error,
}: FormProps) {
  const { t, i18n } = useLingui();
  const needsQuantity = QUANTITY_KINDS.includes(draft.kind);
  const needsSecurity = SECURITY_KINDS.includes(draft.kind);
  // Restrict account choices before submit; the host would reject an invalid kind.
  const allowed = accounts.filter((a) => a.kind === (needsSecurity ? "SECURITIES" : "DEPOSIT"));

  // Changing the operation kind can invalidate the account; select a valid fallback.
  const pickKind = (kind: TransactionKind) => {
    const security = SECURITY_KINDS.includes(kind);
    const fits = accounts.filter((a) => a.kind === (security ? "SECURITIES" : "DEPOSIT"));
    const account = fits.find((a) => a.id === draft.account_id) ?? fits[0];
    onChange({
      ...draft,
      kind,
      account_id: account?.id ?? "",
      currency: account?.currency ?? draft.currency,
      security_id: security ? draft.security_id : null,
    });
  };

  return (
    <FormDialog
      title={draft.id ? t`Edit transaction` : t`New transaction`}
      onClose={onCancel}
      onSubmit={onSubmit}
      busy={pending}
      error={error}
    >
      <Field
        label={t`Kind`}
        options={EDITABLE_TRANSACTION_KINDS.map((value) => ({
          value,
          label: transactionLabel(i18n, value),
        }))}
        value={draft.kind}
        onChange={pickKind}
      />

      <Field label={t`Date`}>
        <input
          type="date"
          value={draft.date}
          onChange={(e) => onChange({ ...draft, date: e.target.value })}
        />
      </Field>

      <Field
        label={t`Account`}
        hint={needsSecurity ? t`Securities account: the instruments sit here` : t`Cash account`}
        options={allowed.map((a) => ({ value: a.id, label: a.name }))}
        value={draft.account_id}
        onChange={(id) => {
          const account = accounts.find((a) => a.id === id);
          onChange({ ...draft, account_id: id, currency: account?.currency ?? draft.currency });
        }}
      />

      {needsSecurity && (
        <Field
          label={t`Instrument`}
          placeholder="—"
          options={securities.map((s) => ({ value: s.id, label: s.symbol }))}
          value={draft.security_id}
          onChange={(id) => onChange({ ...draft, security_id: id || null })}
        />
      )}

      {needsQuantity ? (
        <>
          <Field label={t`Quantity`}>
            <input
              inputMode="decimal"
              value={draft.quantity ?? ""}
              onChange={(e) => onChange({ ...draft, quantity: e.target.value || null })}
            />
          </Field>
          <Field label={t`Price per unit`} hint={t`The amount is quantity × price`}>
            <input
              inputMode="decimal"
              value={draft.price ?? ""}
              onChange={(e) => onChange({ ...draft, price: e.target.value || null })}
            />
          </Field>
        </>
      ) : (
        <Field label={t`Amount`}>
          <input
            inputMode="decimal"
            value={draft.amount ?? ""}
            onChange={(e) => onChange({ ...draft, amount: e.target.value || null })}
          />
        </Field>
      )}

      <Field label={t`Transaction currency`}>
        <input
          value={draft.currency}
          maxLength={3}
          onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
        />
      </Field>

      <FieldPair>
        <Field
          label={t`Commission`}
          hint={t`On a buy it joins the cost basis; on a sell it reduces proceeds`}
        >
          <input
            inputMode="decimal"
            value={draft.fees ?? ""}
            onChange={(e) => onChange({ ...draft, fees: e.target.value || null })}
          />
        </Field>
        <Field label={t`Currency`} hint={t`Empty: the transaction's own`}>
          <input
            value={draft.fee_currency ?? ""}
            maxLength={3}
            placeholder={draft.currency}
            onChange={(e) => onChange({ ...draft, fee_currency: code(e.target.value) })}
          />
        </Field>
      </FieldPair>

      <FieldPair>
        <Field label={t`Tax`}>
          <input
            inputMode="decimal"
            value={draft.taxes ?? ""}
            onChange={(e) => onChange({ ...draft, taxes: e.target.value || null })}
          />
        </Field>
        <Field label={t`Currency`} hint={t`Empty: the transaction's own`}>
          <input
            value={draft.tax_currency ?? ""}
            maxLength={3}
            placeholder={draft.currency}
            onChange={(e) => onChange({ ...draft, tax_currency: code(e.target.value) })}
          />
        </Field>
      </FieldPair>

      <Field
        label={t`Rate to the base currency`}
        hint={t`Fixed at the moment of the trade and never recomputed. Empty means take it from the rate directory`}
      >
        <input
          inputMode="decimal"
          value={draft.fx_rate_to_base ?? ""}
          onChange={(e) => onChange({ ...draft, fx_rate_to_base: e.target.value || null })}
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
