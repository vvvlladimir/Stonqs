import { useLingui } from "@lingui/react/macro";
import { CheckField, Field, FieldSet, FormDialog } from "../../components/ui";
import { ACCOUNT_KIND_LABELS } from "../../lib/kinds";
import type { AccountGroupInput, AccountRow } from "../../lib/types";

export function GroupForm({
  draft,
  accounts,
  onChange,
  onSubmit,
  onCancel,
  pending,
  error,
}: {
  draft: AccountGroupInput;
  accounts: AccountRow[];
  onChange: (draft: AccountGroupInput) => void;
  onSubmit: () => void;
  onCancel: () => void;
  pending: boolean;
  error: Error | null;
}) {
  const { t } = useLingui();
  const toggle = (id: string) => {
    const account_ids = draft.account_ids.includes(id)
      ? draft.account_ids.filter((x) => x !== id)
      : [...draft.account_ids, id];
    onChange({ ...draft, account_ids });
  };

  return (
    <FormDialog
      title={draft.id ? t`Edit group` : t`New group`}
      onClose={onCancel}
      onSubmit={onSubmit}
      busy={pending}
      error={error}
    >
      <Field label={t`Group name`}>
        <input value={draft.name} onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>

      <FieldSet label={t`Accounts`}>
        {accounts.map((a) => (
          <CheckField
            key={a.id}
            label={`${ACCOUNT_KIND_LABELS[a.kind]} · ${a.name}`}
            checked={draft.account_ids.includes(a.id)}
            onChange={() => toggle(a.id)}
          />
        ))}
      </FieldSet>
    </FormDialog>
  );
}
