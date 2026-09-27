import { useState, type ReactNode } from "react";
import { useLingui } from "@lingui/react/macro";
import { useProfiles } from "../../../lib/queries";
import { PasswordDialog } from "../PasswordDialog";
import { SourceKeyDialog } from "./SourceKeyDialog";
import type { MarketSourceRow } from "../../../lib/types";

/** The key dialogs; a profile without a password is asked to set one first (ADR-0048). */
export function useSourceKeys(
  sources: MarketSourceRow[],
  onSaved: () => void,
): { open: (source: string) => void; dialogs: ReactNode } {
  const { t } = useLingui();
  const profiles = useProfiles();
  const isProtected = profiles.data?.profiles.find((p) => p.id === profiles.data?.open)?.protected ?? false;
  const [editing, setEditing] = useState<string | null>(null);
  const [protecting, setProtecting] = useState<string | null>(null);

  const open = (source: string) => (isProtected ? setEditing(source) : setProtecting(source));

  const dialogs = (
    <>
      {protecting && (
        <PasswordDialog
          change={false}
          reason={t`API keys are stored encrypted with this profile's password, so the profile needs one before the first key can be saved.`}
          onClose={() => setProtecting(null)}
          onDone={() => setEditing(protecting)}
        />
      )}
      {editing && (
        <SourceKeyDialog
          source={editing}
          saved={sources.find((s) => s.id === editing)?.has_key ?? false}
          onClose={() => {
            onSaved();
            setEditing(null);
          }}
        />
      )}
    </>
  );

  return { open, dialogs };
}
