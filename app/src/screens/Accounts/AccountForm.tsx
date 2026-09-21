import { useLingui } from "@lingui/react/macro";
import { CheckField, Field, FormDialog, type SelectOption } from "../../components/ui";
import { accountKindLabel } from "../../lib/kinds";
import type { AccountInput, AccountRow } from "../../lib/types";

export function AccountForm({
  draft,
  deposits,
  onChange,
  onSubmit,
  onCancel,
  pending,
  error,
}: {
  draft: AccountInput;
  deposits: AccountRow[];
  onChange: (draft: AccountInput) => void;
  onSubmit: () => void;
  onCancel: () => void;
  pending: boolean;
  error: Error | null;
}) {
  const { t, i18n } = useLingui();

  // Saved account kinds are immutable; changing one would invalidate its transactions.
  // A securities account is offered only when a cash account can back it.
  const canDepot = deposits.length > 0;
  const setKind = (kind: AccountInput["kind"]) =>
    onChange({
      ...draft,
      kind,
      // Keep the reference synchronized so the host never receives an invalid depot.
      reference_account_id:
        kind === "SECURITIES" ? (draft.reference_account_id ?? deposits[0]?.id ?? null) : null,
    });

  const kinds: Array<SelectOption<AccountInput["kind"]>> = [
    { value: "DEPOSIT", label: accountKindLabel(i18n, "DEPOSIT") },
    ...(canDepot ? [{ value: "SECURITIES" as const, label: accountKindLabel(i18n, "SECURITIES") }] : []),
  ];

  return (
    <FormDialog
      title={draft.id ? t`Edit account` : t`New account`}
      onClose={onCancel}
      onSubmit={onSubmit}
      busy={pending}
      error={error}
    >
      <Field
        label={t`Kind`}
        hint={
          draft.id
            ? t`The kind of a saved account never changes: its transactions would stay on an account that no longer accepts them`
            : t`A cash account holds money in one currency; a securities account holds instruments and settles through a cash account`
        }
        options={draft.id ? undefined : kinds}
        value={draft.kind}
        onChange={setKind}
      >
        {draft.id && <input value={accountKindLabel(i18n, draft.kind)} readOnly />}
      </Field>

      <Field label={t`Name`}>
        <input value={draft.name} onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>

      <Field label={t`Currency`} hint={t`The account's settlement currency, not the reporting one`}>
        <input
          value={draft.currency}
          maxLength={3}
          onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
        />
      </Field>

      {draft.kind === "SECURITIES" && (
        <Field
          label={t`Cash account`}
          hint={t`Money for this securities account's trades passes through it`}
          options={deposits.map((a) => ({ value: a.id, label: `${a.name} · ${a.currency}` }))}
          value={draft.reference_account_id}
          onChange={(id) => onChange({ ...draft, reference_account_id: id || null })}
        />
      )}

      <Field label={t`Opened`}>
        <input
          type="date"
          value={draft.opened_at ?? ""}
          onChange={(e) => onChange({ ...draft, opened_at: e.target.value || null })}
        />
      </Field>

      <CheckField
        label={t`Active`}
        checked={draft.is_active}
        onChange={(is_active) => onChange({ ...draft, is_active })}
      />
    </FormDialog>
  );
}
