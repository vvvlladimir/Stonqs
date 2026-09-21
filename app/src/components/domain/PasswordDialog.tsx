import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { MIN_PASSWORD } from "../../lib/profiles";
import { keys, useInvalidate } from "../../lib/queries";
import { Field, FormDialog, SecretInput } from "../ui";

/**
 * Gives the open profile a password, or changes it. A change asks for the current one even though
 * the profile is open: an unattended, unlocked app must not be enough to take it over.
 */
export function PasswordDialog({
  change,
  reason,
  onClose,
  onDone,
}: {
  /** The profile has a password already. */
  change: boolean;
  /** Why a password is asked for now, when it is a step towards something else — saving a key. */
  reason?: string;
  onClose: () => void;
  onDone?: () => void;
}) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const [current, setCurrent] = useState("");
  const [password, setPassword] = useState("");
  const [repeat, setRepeat] = useState("");

  const save = useMutation({
    mutationFn: () => api.profileSetPassword(change ? current : null, password),
    onSuccess: () => {
      invalidate(keys.profiles(), keys.aiProviders(), keys.aiKeyStatus());
      onClose();
      onDone?.();
    },
  });

  const long = password.length >= MIN_PASSWORD;
  const same = password === repeat;

  return (
    <FormDialog
      title={change ? t`Change password` : t`Set a password`}
      submitLabel={reason ? t`Set password and continue` : undefined}
      onClose={onClose}
      onSubmit={() => save.mutate()}
      busy={save.isPending}
      error={save.error}
      ready={long && same && (!change || current.length > 0)}
    >
      {reason && <p className="muted">{reason}</p>}
      {change && (
        <Field label={t`Current password`}>
          <SecretInput autoComplete="current-password" autoFocus value={current} onChange={setCurrent} />
        </Field>
      )}
      <Field
        label={t`New password`}
        hint={
          password.length > 0 && !long
            ? t`Too short: ${password.length} of at least ${MIN_PASSWORD} characters.`
            : change
              ? t`At least ${MIN_PASSWORD} characters.`
              : t`At least ${MIN_PASSWORD} characters. It encrypts the profile's data and keys, and it cannot be recovered: forgotten, it takes all of them with it.`
        }
      >
        <SecretInput
          autoComplete="new-password"
          autoFocus={!change}
          value={password}
          onChange={setPassword}
        />
      </Field>
      <Field
        label={t`Repeat the password`}
        hint={repeat.length > 0 && !same ? t`The two do not match.` : undefined}
      >
        <SecretInput autoComplete="new-password" value={repeat} onChange={setRepeat} />
      </Field>
    </FormDialog>
  );
}
