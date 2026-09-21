import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { CaretRightIcon, LockSimpleIcon, UserCircleIcon } from "@phosphor-icons/react";
import { openProfile } from "../../lib/profiles";
import { ErrorText, Gate, List, ListRow } from "../ui";
import type { Profile, ProfileList } from "../../lib/types";

/**
 * Who is using the app — asked at launch when there is more than one profile. The last one used
 * is already open, so picking it only lets the app through; picking another swaps and reloads.
 */
export function ProfilePicker({ profiles, onPicked }: { profiles: ProfileList; onPicked: () => void }) {
  const { t } = useLingui();
  const pick = useMutation({
    mutationFn: (id: string) => openProfile(id, profiles.open),
    onSuccess: onPicked,
  });

  return (
    <Gate
      title={<Trans>Choose a profile</Trans>}
      lead={t`Each profile keeps its own portfolio, settings and import layouts.`}
    >
      <ProfileRows
        profiles={profiles.profiles}
        lastUsed={profiles.open}
        disabled={pick.isPending}
        onPick={(id) => pick.mutate(id)}
      />
      <ErrorText error={pick.error} />
    </Gate>
  );
}

/** Profiles as tappable cards: the one last used says so, a protected one shows its lock. */
export function ProfileRows({
  profiles,
  lastUsed,
  disabled,
  onPick,
}: {
  profiles: Profile[];
  lastUsed?: string;
  disabled?: boolean;
  onPick: (id: string) => void;
}) {
  const { t } = useLingui();
  return (
    <List variant="cards">
      {profiles.map((profile) => (
        <ListRow
          key={profile.id}
          box
          lead={<UserCircleIcon />}
          title={profile.name}
          sub={profile.id === lastUsed ? t`Last used` : undefined}
          end={
            <>
              {profile.protected && <LockSimpleIcon aria-label={t`Password protected`} />}
              <CaretRightIcon />
            </>
          }
          onClick={disabled ? undefined : () => onPick(profile.id)}
        />
      ))}
    </List>
  );
}
