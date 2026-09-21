import { useLingui } from "@lingui/react/macro";
import { Field, FormDialog } from "../../components/ui";
import type { WatchlistInput } from "../../lib/types";

/** A list's name: a new list, or a rename that keeps its instruments. */
export function ListDialog({
  draft,
  onChange,
  onSubmit,
  onCancel,
  pending,
  error,
}: {
  draft: WatchlistInput;
  onChange: (draft: WatchlistInput) => void;
  onSubmit: () => void;
  onCancel: () => void;
  pending: boolean;
  error: Error | null;
}) {
  const { t } = useLingui();
  return (
    <FormDialog
      title={draft.id ? t`Rename watchlist` : t`New watchlist`}
      onClose={onCancel}
      onSubmit={onSubmit}
      busy={pending}
      error={error}
      ready={draft.name.trim() !== ""}
    >
      <Field label={t`Name`}>
        <input autoFocus value={draft.name} onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>
    </FormDialog>
  );
}
