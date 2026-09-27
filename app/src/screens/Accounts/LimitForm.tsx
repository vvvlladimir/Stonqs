import { useLingui } from "@lingui/react/macro";

import { CheckField, Field, FieldPair, FormDialog } from "../../components/ui";
import type { AccountRow, LimitInput } from "../../lib/types";

export function LimitForm({
  draft,
  accounts,
  onChange,
  onSubmit,
  onClose,
  busy,
  error,
}: {
  draft: LimitInput;
  accounts: AccountRow[];
  onChange: (draft: LimitInput) => void;
  onSubmit: () => void;
  onClose: () => void;
  busy: boolean;
  error: Error | null;
}) {
  const { t } = useLingui();

  return (
    <FormDialog
      title={draft.id ? t`Edit the limit` : t`New limit`}
      onClose={onClose}
      onSubmit={onSubmit}
      busy={busy}
      error={error}
      ready={draft.name.trim() !== "" && draft.amount.trim() !== "" && draft.account_id !== ""}
    >
      <Field label={t`Name`}>
        <input value={draft.name} autoFocus onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>
      <Field
        label={t`Account`}
        options={accounts.map((a) => ({ value: a.id, label: a.name }))}
        value={draft.account_id}
        onChange={(account_id) => onChange({ ...draft, account_id })}
      />
      <FieldPair>
        <Field label={t`Allowance`}>
          <input
            inputMode="decimal"
            value={draft.amount}
            onChange={(e) => onChange({ ...draft, amount: e.target.value })}
          />
        </Field>
        <Field label={t`Currency`}>
          <input
            value={draft.currency}
            onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
          />
        </Field>
      </FieldPair>
      <Field
        label={t`The year opens on`}
        hint={t`Month and day, as MM-DD. Not every allowance year is the calendar one.`}
      >
        <input
          value={draft.year_starts_on}
          placeholder="01-01"
          onChange={(e) => onChange({ ...draft, year_starts_on: e.target.value })}
        />
      </Field>
      <CheckField
        label={t`Withdrawals give allowance back`}
        hint={t`On for a flexible allowance. Off, money taken out still counts as paid in.`}
        checked={draft.withdrawals_restore}
        onChange={(withdrawals_restore) => onChange({ ...draft, withdrawals_restore })}
      />
    </FormDialog>
  );
}
