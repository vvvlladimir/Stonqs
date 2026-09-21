import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { LockIcon, LockOpenIcon, ShieldCheckIcon, ShieldWarningIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { restart } from "../../lib/profiles";
import { keys, useInvalidate } from "../../lib/queries";
import { PasswordDialog } from "../../components/domain/PasswordDialog";
import {
  Buttons,
  CheckField,
  ErrorText,
  Field,
  FormDialog,
  ListRow,
  Panel,
  SecretInput,
} from "../../components/ui";
import type { ProfileList } from "../../lib/types";

/** The open profile's password: set, change, remove, lock now, and whether this device remembers it. */
export function ProfileSecurity({ profiles }: { profiles: ProfileList }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const [dialog, setDialog] = useState<"set" | "change" | "remove" | null>(null);
  const isProtected = profiles.profiles.find((p) => p.id === profiles.open)?.protected ?? false;

  const lock = useMutation({ mutationFn: api.profileLock, onSuccess: restart });
  const remember = useMutation({
    mutationFn: api.profileRemember,
    onSuccess: () => invalidate(keys.profiles()),
  });

  return (
    <Panel
      title={t`Password`}
      info={t`The password encrypts this profile's data and API keys and is asked when the profile opens.`}
    >
      <ListRow
        box
        wrap
        lead={isProtected ? <ShieldCheckIcon /> : <ShieldWarningIcon />}
        title={isProtected ? t`Password set` : t`No password`}
        sub={
          isProtected
            ? t`This profile's data and API keys are encrypted with it.`
            : t`This profile's data is stored unencrypted. Set a password before saving an API key.`
        }
        end={
          isProtected ? (
            <button className="btn btn--ghost btn--sm" onClick={() => setDialog("change")}>
              <Trans>Change password</Trans>
            </button>
          ) : (
            <button className="btn btn--sm" onClick={() => setDialog("set")}>
              <LockIcon /> <Trans>Set a password</Trans>
            </button>
          )
        }
      />
      {isProtected && (
        <>
          <CheckField
            label={t`Remember on this device`}
            hint={t`This device opens the profile without asking; anyone using it can too.`}
            checked={profiles.remembered}
            disabled={remember.isPending}
            onChange={(on) => remember.mutate(on)}
          />
          <Buttons>
            <button
              className="btn btn--ghost btn--sm"
              disabled={lock.isPending}
              onClick={() => lock.mutate()}
            >
              <LockIcon /> <Trans>Lock now</Trans>
            </button>
            <button className="btn btn--danger btn--sm" onClick={() => setDialog("remove")}>
              <LockOpenIcon /> <Trans>Remove password</Trans>
            </button>
          </Buttons>
        </>
      )}
      <ErrorText error={lock.error ?? remember.error} />

      {(dialog === "set" || dialog === "change") && (
        <PasswordDialog change={dialog === "change"} onClose={() => setDialog(null)} />
      )}
      {dialog === "remove" && <RemoveDialog onClose={() => setDialog(null)} />}
    </Panel>
  );
}

/** Removing the password removes the keys it protected: a key is kept only behind one. */
function RemoveDialog({ onClose }: { onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const [current, setCurrent] = useState("");
  const remove = useMutation({
    mutationFn: () => api.profileRemovePassword(current),
    onSuccess: () => {
      invalidate(keys.profiles(), keys.aiProviders(), keys.aiKeyStatus());
      onClose();
    },
  });

  return (
    <FormDialog
      title={t`Remove password`}
      onClose={onClose}
      onSubmit={() => remove.mutate()}
      busy={remove.isPending}
      error={remove.error}
      ready={current.length > 0}
      submitLabel={t`Remove password and keys`}
    >
      <Field
        label={t`Current password`}
        hint={t`The profile's data is decrypted, and every API key saved in it is deleted.`}
      >
        <SecretInput autoComplete="current-password" autoFocus value={current} onChange={setCurrent} />
      </Field>
    </FormDialog>
  );
}
