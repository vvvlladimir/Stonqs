import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { plural } from "@lingui/core/macro";
import { CpuIcon, KeyIcon } from "@phosphor-icons/react";
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
export function KeysPanel({
  providers,
  extraModels,
  onSaveModels,
}: {
  providers: AiProvider[];
  /** Ids the user added per provider, offered in the chat's model picker. */
  extraModels: Record<string, string[]>;
  onSaveModels: (provider: string, ids: string[]) => void;
}) {
  const { t } = useLingui();
  const name = useProviderName();
  const [editing, setEditing] = useState<string | null>(null);
  const [models, setModels] = useState<string | null>(null);
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
          models={extraModels[provider.id]?.length ?? 0}
          onEdit={() => edit(provider.id)}
          onModels={() => setModels(provider.id)}
        />
      ))}
      {models && (
        <ModelsDialog
          name={name(models, providers.find((p) => p.id === models)?.label)}
          ids={extraModels[models] ?? []}
          onSave={(ids) => {
            onSaveModels(models, ids);
            setModels(null);
          }}
          onClose={() => setModels(null)}
        />
      )}
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

function KeyRow({
  provider,
  name,
  models,
  onEdit,
  onModels,
}: {
  provider: string;
  name: string;
  /** How many model ids the user added for this provider. */
  models: number;
  onEdit: () => void;
  onModels: () => void;
}) {
  const { t } = useLingui();
  const status = useAiKeyStatus(provider);
  const saved = status.data ?? false;
  const added = plural(models, { one: "# own model", other: "# own models" });

  return (
    <ListRow
      box
      lead={<KeyIcon />}
      title={name}
      sub={
        models > 0
          ? saved
            ? t`A key is saved · ${added}`
            : t`No key saved · ${added}`
          : saved
            ? t`A key is saved`
            : t`No key saved`
      }
      end={
        <>
          <Badge tone={saved ? "in" : "neutral"}>{saved ? t`Connected` : t`Not connected`}</Badge>
          <button
            type="button"
            className="iconbtn iconbtn--sm"
            aria-label={t`Models for ${name}`}
            data-tip={t`Add models to the picker`}
            onClick={onModels}
          >
            <CpuIcon />
          </button>
          <button type="button" className="btn btn--sm btn--ghost" onClick={onEdit}>
            {saved ? <Trans>Replace</Trans> : <Trans>Connect</Trans>}
          </button>
        </>
      }
    />
  );
}

/** Model ids typed in by hand, one per line. They join the chat's picker after the provider's own
 * shortlist, so a model the shortlist leaves out (or the catalogue does not list) is reachable. */
function ModelsDialog({
  name,
  ids,
  onSave,
  onClose,
}: {
  name: string;
  ids: string[];
  onSave: (ids: string[]) => void;
  onClose: () => void;
}) {
  const { t } = useLingui();
  const [text, setText] = useState(ids.join("\n"));
  const parsed = [
    ...new Set(
      text
        .split(/[\n,]/)
        .map((id) => id.trim())
        .filter((id) => id !== ""),
    ),
  ];

  return (
    <FormDialog title={t`Models for ${name}`} onClose={onClose} onSubmit={() => onSave(parsed)} ready>
      <Field
        label={t`Model ids`}
        hint={t`One per line, exactly as the provider spells it. They are offered in the chat's model picker after the provider's own.`}
      >
        <textarea
          autoFocus
          rows={5}
          spellCheck={false}
          // eslint-disable-next-line lingui/no-unlocalized-strings -- a model id, not text
          placeholder="gemini-3.1-pro-preview"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
      </Field>
    </FormDialog>
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
