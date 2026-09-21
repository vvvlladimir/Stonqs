import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { openProfile, restart } from "../../lib/profiles";
import { CheckField, ErrorText, Field, Form, Gate, GateSection, SecretInput } from "../ui";
import { ProfileRows } from "./ProfilePicker";
import type { ProfileList } from "../../lib/types";

/**
 * The open profile has a password and nothing of it is readable until it is typed. The other
 * profiles stay one press away, so a locked profile never traps the app.
 */
export function ProfileLock({ profiles }: { profiles: ProfileList }) {
  const { t } = useLingui();
  const [password, setPassword] = useState("");
  const [remember, setRemember] = useState(false);
  const name = profiles.profiles.find((p) => p.id === profiles.open)?.name ?? "";
  const others = profiles.profiles.filter((p) => p.id !== profiles.open);

  const unlock = useMutation({
    mutationFn: () => api.profileUnlock(password, remember),
    onSuccess: restart,
  });
  const leave = useMutation({ mutationFn: (id: string) => openProfile(id, profiles.open) });

  return (
    <Gate
      title={<Trans>Unlock {name}</Trans>}
      lead={t`Enter the password to open this profile.`}
      foot={
        others.length > 0 && (
          <GateSection title={t`Other profiles`}>
            <ProfileRows profiles={others} disabled={leave.isPending} onPick={(id) => leave.mutate(id)} />
            <ErrorText error={leave.error} />
          </GateSection>
        )
      }
    >
      <Form
        onSubmit={() => unlock.mutate()}
        busy={unlock.isPending}
        error={unlock.error}
        ready={password.length > 0}
        submitLabel={t`Unlock`}
        busyLabel={t`Unlocking…`}
      >
        <Field label={t`Password`}>
          <SecretInput autoComplete="current-password" autoFocus value={password} onChange={setPassword} />
        </Field>
        <CheckField
          label={t`Remember on this device`}
          hint={t`This device opens the profile without asking; anyone using it can too.`}
          checked={remember}
          onChange={setRemember}
        />
      </Form>
    </Gate>
  );
}
