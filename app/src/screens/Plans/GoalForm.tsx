import { useLingui } from "@lingui/react/macro";
import { CheckField, Field, FieldPair, FormDialog } from "../../components/ui";
import type { AccountRow, GoalInput } from "../../lib/types";

export function GoalForm({
  draft,
  accounts,
  onChange,
  onSubmit,
  onClose,
  busy,
  error,
}: {
  draft: GoalInput;
  accounts: AccountRow[];
  onChange: (draft: GoalInput) => void;
  onSubmit: () => void;
  onClose: () => void;
  busy: boolean;
  error: Error | null;
}) {
  const { t } = useLingui();
  const toggle = (id: string, on: boolean) =>
    onChange({
      ...draft,
      accounts: on ? [...draft.accounts, id] : draft.accounts.filter((a) => a !== id),
    });

  return (
    <FormDialog
      title={draft.id ? t`Edit the goal` : t`New goal`}
      onClose={onClose}
      onSubmit={onSubmit}
      busy={busy}
      error={error}
      ready={draft.name.trim() !== "" && draft.target_amount.trim() !== ""}
    >
      <Field label={t`Name`}>
        <input value={draft.name} autoFocus onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>
      <FieldPair>
        <Field label={t`Target amount`}>
          <input
            inputMode="decimal"
            value={draft.target_amount}
            onChange={(e) => onChange({ ...draft, target_amount: e.target.value })}
          />
        </Field>
        <Field label={t`Currency`}>
          <input
            value={draft.currency}
            onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
          />
        </Field>
      </FieldPair>
      <FieldPair>
        <Field label={t`By`} hint={t`Leave empty to ask when the monthly amount would arrive instead.`}>
          <input
            type="date"
            value={draft.target_date ?? ""}
            onChange={(e) => onChange({ ...draft, target_date: e.target.value || null })}
          />
        </Field>
        <Field label={t`Paid in monthly`} hint={t`Optional: what you actually put aside.`}>
          <input
            inputMode="decimal"
            value={draft.monthly_amount ?? ""}
            onChange={(e) => onChange({ ...draft, monthly_amount: e.target.value || null })}
          />
        </Field>
      </FieldPair>
      <Field
        label={t`Expected return, % a year`}
        hint={t`An assumption, never this portfolio's measured return. Leave empty for pure saving.`}
      >
        <input
          inputMode="decimal"
          value={draft.expected_return ?? ""}
          onChange={(e) => onChange({ ...draft, expected_return: e.target.value || null })}
        />
      </Field>
      <Field label={t`Accounts that count`} hint={t`None ticked means the whole portfolio.`}>
        <div className="smenu">
          {accounts.map((account) => (
            <CheckField
              key={account.id}
              label={account.name}
              checked={draft.accounts.includes(account.id)}
              onChange={(on) => toggle(account.id, on)}
            />
          ))}
        </div>
      </Field>
    </FormDialog>
  );
}
