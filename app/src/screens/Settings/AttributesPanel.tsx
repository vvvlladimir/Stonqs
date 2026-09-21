import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { ATTRIBUTE_KINDS, attributeKindLabel } from "../../lib/kinds";
import { affects, useAttributeDefs, useInvalidate } from "../../lib/queries";
import {
  Empty,
  ErrorText,
  Field,
  FormDialog,
  List,
  ListRow,
  Panel,
  Pending,
  QueryError,
  Tag,
} from "../../components/ui";
import type { AttributeDefInput, SecurityAttributeDef } from "../../lib/types";

const EMPTY: AttributeDefInput = { id: null, name: "", kind: "TEXT", unit: null, position: 0 };

/**
 * The columns the user adds to the instrument directory: TER, country of risk, replication.
 * The values themselves are filled in on the instrument, because that is where they belong.
 */
export function AttributesPanel() {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const defs = useAttributeDefs();
  const [draft, setDraft] = useState<AttributeDefInput | null>(null);

  const save = useMutation({
    mutationFn: api.attributeDefSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.securities);
    },
  });
  const remove = useMutation({
    mutationFn: api.attributeDefDelete,
    onSuccess: () => invalidate(...affects.securities),
  });

  if (defs.isError) return <QueryError error={defs.error} />;
  if (defs.isPending) return <Pending />;

  const edit = (def: SecurityAttributeDef) =>
    setDraft({ id: def.id, name: def.name, kind: def.kind, unit: def.unit, position: def.position });

  return (
    <Panel
      title={t`Instrument attributes`}
      info={t`Your own fields for instruments, offered on every instrument's form.`}
      tools={
        <button
          className="btn btn--ghost btn--sm"
          onClick={() => setDraft({ ...EMPTY, position: defs.data.length })}
        >
          <PlusIcon /> <Trans>Add attribute</Trans>
        </button>
      }
    >
      <ErrorText error={remove.error} />

      {draft && (
        <FormDialog
          title={draft.id ? t`Edit attribute` : t`New attribute`}
          onClose={() => setDraft(null)}
          onSubmit={() => save.mutate(draft)}
          busy={save.isPending}
          error={save.error}
          ready={draft.name.trim() !== ""}
        >
          <Field label={t`Name`}>
            <input
              autoFocus
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </Field>

          <Field
            label={t`Kind`}
            hint={
              draft.id
                ? t`The kind is fixed once the attribute exists: changing it would leave the values already filled in unreadable.`
                : t`A rate is a number with "%" as its unit — there is no percent kind to guess at.`
            }
            disabled={draft.id !== null}
            options={ATTRIBUTE_KINDS.map((kind) => ({ value: kind, label: attributeKindLabel(i18n, kind) }))}
            value={draft.kind}
            onChange={(kind) => setDraft({ ...draft, kind })}
          />

          <Field label={t`Unit`} hint={t`Shown after the value; never part of it`}>
            <input
              value={draft.unit ?? ""}
              onChange={(e) => setDraft({ ...draft, unit: e.target.value || null })}
            />
          </Field>
        </FormDialog>
      )}

      {defs.data.length === 0 ? (
        <Empty title={t`No attributes yet`}>
          <Trans>
            An attribute is a column of your own on every instrument — the TER of a fund, the country whose
            risk it carries, the date you first bought it. Nothing is computed from them yet; they are there
            to be read.
          </Trans>
        </Empty>
      ) : (
        <List>
          {defs.data.map((def) => (
            <ListRow
              key={def.id}
              title={def.name}
              sub={<Tag>{attributeKindLabel(i18n, def.kind)}</Tag>}
              meta={def.unit ?? undefined}
              onClick={() => edit(def)}
              end={
                <button
                  type="button"
                  className="iconbtn iconbtn--sm iconbtn--danger"
                  disabled={remove.isPending}
                  onClick={(e) => {
                    e.stopPropagation();
                    remove.mutate(def.id);
                  }}
                >
                  <Trans>Delete</Trans>
                </button>
              }
            />
          ))}
        </List>
      )}
    </Panel>
  );
}
