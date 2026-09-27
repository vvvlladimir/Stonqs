import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { Field, FormDialog, SecretInput } from "../../components/ui";

/** Sent with the next load to the asking reader only, and not kept anywhere. */
export function FilePasswordDialog({
  plugin,
  tried,
  busy,
  onSubmit,
  onClose,
}: {
  /** The name of the plugin whose reader asked. */
  plugin: string;
  /** A password was already given and refused. */
  tried: boolean;
  busy: boolean;
  onSubmit: (password: string) => void;
  onClose: () => void;
}) {
  const { t } = useLingui();
  const [password, setPassword] = useState("");

  return (
    <FormDialog
      title={t`This file is password-protected`}
      submitLabel={t`Open`}
      onClose={onClose}
      onSubmit={() => onSubmit(password)}
      busy={busy}
      ready={password.length > 0}
    >
      <Field
        label={t`Password`}
        hint={
          tried
            ? t`That password did not open it. Try again.`
            : t`The ${plugin} plugin reads this file and needs its password. It is used once and not saved.`
        }
      >
        <SecretInput autoComplete="off" autoFocus value={password} onChange={setPassword} />
      </Field>
    </FormDialog>
  );
}
