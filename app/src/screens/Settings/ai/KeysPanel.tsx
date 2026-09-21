import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { KeyIcon } from "@phosphor-icons/react";
import { api } from "../../../lib/api";
import { useProviderName } from "../../../lib/ai";
import { keys, useAiKeyStatus, useInvalidate, useProfiles } from "../../../lib/queries";
import { PasswordDialog } from "../../../components/domain/PasswordDialog";
import type { AiProvider } from "../../../lib/types";
import { Badge, Field, FormDialog, ListRow, Panel, SecretInput } from "../../../components/ui";

/**
 * One row per provider: whether a key is saved, and one way to change it. A panel per provider
 * pushed everything else off the screen, and the answer this screen owes is the same for each —
 * connected or not.
 *
 * The key never comes back: `ai_key_status` reports whether one is saved and nothing more, so
 * the field is always empty and always a replacement.
 */
export function KeysPanel({ providers }: { providers: AiProvider[] }) {
  const { t } = useLingui();
  const name = useProviderName();
  const [editing, setEditing] = useState<string | null>(null);
  // A key is kept only behind the profile's password, so the first one asks for that first and
  // then carries on to the key it was opened for.
  const [protecting, setProtecting] = useState<string | null>(null);
  const profiles = useProfiles();
  const isProtected = profiles.data?.profiles.find((p) => p.id === profiles.data?.open)?.protected ?? false;
  const edit = (provider: string) => (isProtected ? setEditing(provider) : setProtecting(provider));

  return (
    <Panel
      title={t`Provider keys`}
      info={t`Keys are stored encrypted with this profile's password, never in its database, and are never shown again.`}
    >
      {providers.map((provider) => (
        <KeyRow
          key={provider.id}
          provider={provider.id}
          name={name(provider.id, provider.label)}
          onEdit={() => edit(provider.id)}
        />
      ))}
      {protecting && (
        <PasswordDialog
          change={false}
          reason={t`API keys are stored encrypted with this profile's password, so the profile needs one before the first key for ${name(protecting, providers.find((p) => p.id === protecting)?.label)} can be saved. The password also encrypts the profile's data, and it is asked once when the profile opens.`}
          onClose={() => setProtecting(null)}
          onDone={() => setEditing(protecting)}
        />
      )}
      {editing && (
        <KeyDialog
          provider={editing}
          name={name(editing, providers.find((p) => p.id === editing)?.label)}
          onClose={() => setEditing(null)}
        />
      )}
    </Panel>
  );
}

function KeyRow({ provider, name, onEdit }: { provider: string; name: string; onEdit: () => void }) {
  const { t } = useLingui();
  const status = useAiKeyStatus(provider);
  const saved = status.data ?? false;

  return (
    <ListRow
      box
      lead={<KeyIcon />}
      title={name}
      sub={saved ? t`A key is saved` : t`No key saved`}
      end={
        <>
          <Badge tone={saved ? "in" : "neutral"}>{saved ? t`Connected` : t`Not connected`}</Badge>
          <button type="button" className="btn btn--sm btn--ghost" onClick={onEdit}>
            {saved ? <Trans>Replace</Trans> : <Trans>Connect</Trans>}
          </button>
        </>
      }
    />
  );
}

/** Entering a key is a dialog rather than a field on the page: it is a secret being typed, and
 * the page around it is a list of switches. */
function KeyDialog({ provider, name, onClose }: { provider: string; name: string; onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const status = useAiKeyStatus(provider);
  const [key, setKey] = useState("");

  const done = () => {
    invalidate(keys.aiKeyStatus(provider), keys.aiProviders());
    onClose();
  };
  const save = useMutation({ mutationFn: () => api.aiKeySave(provider, key.trim()), onSuccess: done });
  const remove = useMutation({ mutationFn: () => api.aiKeyDelete(provider), onSuccess: done });

  return (
    <FormDialog
      title={t`Key for ${name}`}
      onClose={onClose}
      onSubmit={() => save.mutate()}
      busy={save.isPending}
      error={save.error ?? remove.error}
      ready={key.trim().length > 0}
      submitLabel={t`Save key`}
      busyLabel={t`Saving…`}
      lead={
        status.data ? (
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
        <SecretInput
          autoComplete="off"
          autoFocus
          placeholder={status.data ? t`Replace the saved key…` : "sk-…"}
          value={key}
          onChange={setKey}
        />
      </Field>
    </FormDialog>
  );
}
