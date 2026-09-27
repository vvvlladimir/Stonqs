import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { Field, FormDialog, Modal } from "../../../components/ui";
import type { Dashboard as Board } from "../../../lib/uiState";

export function BoardName({
  action,
  current,
  onClose,
  onSave,
}: {
  action: "new" | "rename";
  current: string;
  onClose: () => void;
  onSave: (name: string) => void;
}) {
  const { t } = useLingui();
  const [name, setName] = useState(current || t`New dashboard`);
  return (
    <FormDialog
      title={action === "new" ? t`New dashboard` : t`Rename dashboard`}
      onClose={onClose}
      onSubmit={() => onSave(name.trim() || current || t`Dashboard`)}
      submitLabel={action === "new" ? t`Create` : t`Save`}
    >
      <Field
        label={t`Name`}
        hint={t`Dashboards are independent: each keeps its own widgets and their settings.`}
      >
        <input type="text" autoFocus value={name} onChange={(e) => setName(e.target.value)} />
      </Field>
    </FormDialog>
  );
}

/** The last board is never deleted: the screen would have nothing to show. */
export function DeleteBoard({
  board,
  boards,
  onClose,
  onDelete,
}: {
  board: Board;
  boards: Board[];
  onClose: () => void;
  onDelete: () => void;
}) {
  const { t } = useLingui();
  const last = boards.length === 1;
  return (
    <Modal
      title={last ? t`Cannot delete` : t`Delete dashboard`}
      onClose={onClose}
      foot={
        <>
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            {last ? t`Close` : t`Cancel`}
          </button>
          {!last && (
            <button type="button" className="btn btn--danger" onClick={onDelete}>
              <Trans>Delete</Trans>
            </button>
          )}
        </>
      }
    >
      {last ? (
        <p className="muted">
          <Trans>The last dashboard stays. Rename it, or clear its layout.</Trans>
        </p>
      ) : (
        <p className="muted">
          <Trans>
            Delete "{board.name}" and its {board.widgets.length} widgets? Accounts and transactions are
            untouched — a dashboard stores only a layout.
          </Trans>
        </p>
      )}
    </Modal>
  );
}
