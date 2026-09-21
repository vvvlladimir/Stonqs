import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { CloudArrowDownIcon, PlusIcon } from "@phosphor-icons/react";
import { api } from "../../../lib/api";
import { keys, useInvalidate, useMarketCustom, useMarketSources, useProfiles } from "../../../lib/queries";
import { PasswordDialog } from "../../../components/domain/PasswordDialog";
import type { CustomSource, MarketSourceRow } from "../../../lib/types";
import { Badge, Empty, Field, FormDialog, ListRow, Panel, SecretInput } from "../../../components/ui";
import { CustomSourceDialog } from "./CustomSourceDialog";

const CAPABILITY = {
  quotes: msg`quotes`,
  search: msg`search`,
  listings: msg`venues`,
  fx_rates: msg`exchange rates`,
} as const;

/** Every source the app can ask, which of them it does, and the keys that open the rest. */
export function SourcesPanel() {
  const { t } = useLingui();
  const sources = useMarketSources();
  const custom = useMarketCustom();
  const invalidate = useInvalidate();
  const profiles = useProfiles();
  const isProtected = profiles.data?.profiles.find((p) => p.id === profiles.data?.open)?.protected ?? false;
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [protecting, setProtecting] = useState<string | null>(null);
  const [editingCustom, setEditingCustom] = useState<CustomSource | "new" | null>(null);

  const refresh = () => invalidate(keys.marketSources(), keys.marketCustom(), keys.quoteProviders());
  const switchOne = useMutation({
    mutationFn: ({ id, on }: { id: string; on: boolean }) => api.marketSourceSwitch(id, on),
    onSuccess: refresh,
  });
  const openKey = (id: string) => (isProtected ? setEditingKey(id) : setProtecting(id));

  return (
    <>
      <Panel
        title={t`Data sources`}
        info={t`Where quotes and exchange rates come from; an instrument's own source is asked first.`}
      >
        {(sources.data ?? []).map((row) => (
          <SourceRow
            key={row.id}
            row={row}
            busy={switchOne.isPending}
            onSwitch={(on) => switchOne.mutate({ id: row.id, on })}
            onKey={() => openKey(row.id)}
          />
        ))}
      </Panel>

      <Panel
        title={t`Your own sources`}
        info={t`A price feed you describe: its address and where the dates and closes are in the answer.`}
        tools={
          <button type="button" className="btn btn--sm btn--ghost" onClick={() => setEditingCustom("new")}>
            <PlusIcon /> <Trans>Add source</Trans>
          </button>
        }
      >
        {(custom.data ?? []).length === 0 ? (
          <Empty title={t`No sources of your own`}>
            <Trans>Any JSON or CSV price endpoint can be added without an update of the app.</Trans>
          </Empty>
        ) : (
          (custom.data ?? []).map((source) => (
            <ListRow
              key={source.id}
              box
              lead={<CloudArrowDownIcon />}
              title={source.label}
              sub={`${source.role === "fx" ? t`exchange rates` : t`quotes`} · ${source.url}`}
              end={
                <button
                  type="button"
                  className="btn btn--sm btn--ghost"
                  onClick={() => setEditingCustom(source)}
                >
                  <Trans>Edit</Trans>
                </button>
              }
            />
          ))
        )}
      </Panel>

      {protecting && (
        <PasswordDialog
          change={false}
          reason={t`API keys are stored encrypted with this profile's password, so the profile needs one before the first key can be saved.`}
          onClose={() => setProtecting(null)}
          onDone={() => setEditingKey(protecting)}
        />
      )}
      {editingKey && (
        <SourceKeyDialog
          source={editingKey}
          saved={sources.data?.find((s) => s.id === editingKey)?.has_key ?? false}
          onClose={() => {
            refresh();
            setEditingKey(null);
          }}
        />
      )}
      {editingCustom && (
        <CustomSourceDialog
          source={editingCustom === "new" ? null : editingCustom}
          onKey={openKey}
          onClose={() => {
            refresh();
            setEditingCustom(null);
          }}
        />
      )}
    </>
  );
}

function SourceRow({
  row,
  busy,
  onSwitch,
  onKey,
}: {
  row: MarketSourceRow;
  busy: boolean;
  onSwitch: (on: boolean) => void;
  onKey: () => void;
}) {
  const { t, i18n } = useLingui();
  const roles = row.capabilities.map((c) => i18n._(CAPABILITY[c])).join(", ");
  const needsKey = row.wanted && !row.active;

  return (
    <ListRow
      box
      title={row.id}
      sub={roles}
      end={
        <>
          <Badge tone={row.active ? "in" : needsKey ? "warn" : "neutral"}>
            {row.active ? t`On` : needsKey ? t`Needs a key` : t`Off`}
          </Badge>
          {row.key !== "none" && (
            <button type="button" className="btn btn--sm btn--ghost" onClick={onKey}>
              {row.has_key ? <Trans>Replace key</Trans> : <Trans>Add key</Trans>}
            </button>
          )}
          <button
            type="button"
            className="btn btn--sm btn--ghost"
            disabled={busy}
            onClick={() => onSwitch(!row.wanted)}
          >
            {row.wanted ? <Trans>Turn off</Trans> : <Trans>Turn on</Trans>}
          </button>
        </>
      }
    />
  );
}

/** The key never comes back: the field is always empty and always a replacement. */
function SourceKeyDialog({
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
