import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { PencilSimpleIcon, PlusIcon, SignInIcon, TrashIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { openProfile, restart } from "../../lib/profiles";
import { keys, useInvalidate, useProfiles } from "../../lib/queries";
import { ErrorText, Field, FormDialog, List, ListRow, Panel, SecretInput } from "../../components/ui";
import type { Profile } from "../../lib/types";
import { ProfileSecurity } from "./ProfileSecurity";

/** Every profile on this device: open one, add, rename, and delete the open one. */
export function ProfilesPanel() {
  const { t } = useLingui();
  const profiles = useProfiles();
  const [naming, setNaming] = useState<Profile | "new" | null>(null);
  const [deleting, setDeleting] = useState<Profile | null>(null);

  const open = useMutation({
    mutationFn: (id: string) => openProfile(id, profiles.data?.open ?? ""),
  });

  const list = profiles.data?.profiles ?? [];
  const current = profiles.data?.open;

  return (
    <>
      <Panel
        title={t`Profiles`}
        info={t`Each profile keeps its own portfolio, settings and import layouts.`}
        tools={
          <button className="btn btn--ghost btn--sm" onClick={() => setNaming("new")}>
            <PlusIcon /> <Trans>New profile</Trans>
          </button>
        }
      >
        <List>
          {list.map((profile) => {
            const isOpen = profile.id === current;
            return (
              <ListRow
                key={profile.id}
                title={profile.name}
                sub={
                  [isOpen ? t`Open` : null, profile.protected ? t`Password protected` : null]
                    .filter(Boolean)
                    .join(" · ") || undefined
                }
                end={
                  isOpen ? undefined : (
                    <button
                      className="btn btn--ghost btn--sm"
                      disabled={open.isPending}
                      onClick={() => open.mutate(profile.id)}
                    >
                      <SignInIcon /> <Trans>Switch</Trans>
                    </button>
                  )
                }
                showActions
                actions={
                  <>
                    <button
                      className="iconbtn iconbtn--sm"
                      title={t`Rename`}
                      onClick={() => setNaming(profile)}
                    >
                      <PencilSimpleIcon />
                    </button>
                    {/* Only the open profile can be deleted: deleting one takes being in it. */}
                    {isOpen && (
                      <button
                        className="iconbtn iconbtn--sm iconbtn--danger"
                        title={list.length < 2 ? t`The last profile cannot be deleted` : t`Delete`}
                        disabled={list.length < 2}
                        onClick={() => setDeleting(profile)}
                      >
                        <TrashIcon />
                      </button>
                    )}
                  </>
                }
              />
            );
          })}
        </List>
        <ErrorText error={open.error} />
      </Panel>

      {profiles.data && <ProfileSecurity profiles={profiles.data} />}

      {naming !== null && (
        <NameDialog profile={naming === "new" ? null : naming} onClose={() => setNaming(null)} />
      )}

      {deleting && <DeleteDialog profile={deleting} onClose={() => setDeleting(null)} />}
    </>
  );
}

/** Creates a profile, or renames one. Creating does not switch to it. */
function NameDialog({ profile, onClose }: { profile: Profile | null; onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const [name, setName] = useState(profile?.name ?? "");
  const save = useMutation({
    mutationFn: () => (profile ? api.profileRename(profile.id, name) : api.profileCreate(name)),
    onSuccess: () => {
      invalidate(keys.profiles());
      onClose();
    },
  });

  return (
    <FormDialog
      title={profile ? t`Rename profile` : t`New profile`}
      onClose={onClose}
      onSubmit={() => save.mutate()}
      ready={name.trim() !== ""}
      busy={save.isPending}
      error={save.error}
      submitLabel={profile ? undefined : t`Create`}
    >
      <Field
        label={t`Name`}
        hint={profile ? undefined : t`A new profile starts empty; switch to it to set it up.`}
      >
        <input type="text" autoFocus value={name} onChange={(e) => setName(e.target.value)} />
      </Field>
    </FormDialog>
  );
}

/** Deletes the open profile. A protected one asks for its password again, the way a password
 * change does: an unlocked, unattended app must not be enough to wipe it. */
function DeleteDialog({ profile, onClose }: { profile: Profile; onClose: () => void }) {
  const { t } = useLingui();
  const [password, setPassword] = useState("");
  const remove = useMutation({
    mutationFn: () => api.profileDelete(profile.protected ? password : null),
    onSuccess: restart,
  });

  return (
    <FormDialog
      title={t`Delete profile`}
      onClose={onClose}
      onSubmit={() => remove.mutate()}
      busy={remove.isPending}
      error={remove.error}
      ready={!profile.protected || password.length > 0}
      submitLabel={t`Delete profile`}
    >
      <p className="muted">
        <Trans>
          Delete "{profile.name}" with every account, transaction, quote and setting in it? This cannot be
          undone. The app then opens another profile.
        </Trans>
      </p>
      {profile.protected && (
        <Field label={t`Password`}>
          <SecretInput autoComplete="current-password" autoFocus value={password} onChange={setPassword} />
        </Field>
      )}
    </FormDialog>
  );
}
