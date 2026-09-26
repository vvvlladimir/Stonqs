import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../../lib/api";
import { Field, FormDialog, SecretInput } from "../../ui";

/** The key never comes back: the field is always empty and always a replacement. */
export function SourceKeyDialog({
  source,
  saved,
  onClose,
}: {
  source: string;
  saved: boolean;
  onClose: () => void;
}) {
  const { t } = useLingui();
  const [key, setKey] = useState("");
  const save = useMutation({ mutationFn: () => api.marketKeySave(source, key.trim()), onSuccess: onClose });
  const remove = useMutation({ mutationFn: () => api.marketKeyDelete(source), onSuccess: onClose });

  return (
    <FormDialog
      title={t`Key for ${source}`}
      onClose={onClose}
      onSubmit={() => save.mutate()}
      busy={save.isPending}
      error={save.error ?? remove.error}
      ready={key.trim().length > 0}
      submitLabel={t`Save key`}
      busyLabel={t`Saving…`}
      lead={
        saved ? (
          <button
            type="button"
            className="btn btn--sm btn--danger"
            disabled={remove.isPending}
            onClick={() => remove.mutate()}
          >
            <Trans>Delete key</Trans>
          </button>
        ) : undefined
      }
    >
      <Field
        label={t`Key`}
        hint={t`It is sealed with the profile's password. The app never reads it back into this screen.`}
      >
        <SecretInput autoComplete="off" autoFocus value={key} onChange={setKey} />
      </Field>
    </FormDialog>
  );
}
