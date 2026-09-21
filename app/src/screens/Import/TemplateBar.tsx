import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowCounterClockwiseIcon, FloppyDiskIcon, TrashIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { keys } from "../../lib/queries";
import { Buttons, Choice, ErrorText, Field, FormDialog } from "../../components/ui";
import type { ImportMapping, ImportTemplate, ParseConfig } from "../../lib/types";

/**
 * Reusable broker layouts, as one line: pick one, save what is on screen as a new one,
 * drop the one picked. It sits above the step's only panel because a template answers
 * every question below it at once — reaching for it later means redoing the work.
 */
export function TemplateBar({
  config,
  mapping,
  templates,
  applied,
  onApplied,
  onForget,
}: {
  config: ParseConfig;
  mapping: ImportMapping;
  templates: ImportTemplate[];
  /** Name of the template this file was laid out with, or "" for a hand-made mapping. */
  applied: string;
  /** Lays the file out by this template; "" puts back what the core detected. */
  onApplied: (name: string) => void;
  /** Drops the name without touching the layout: the template is gone, the work is not. */
  onForget: () => void;
}) {
  const { t } = useLingui();
  const queryClient = useQueryClient();
  const [name, setName] = useState<string | null>(null);

  const save = useMutation({
    mutationFn: (title: string) => api.importTemplateSave(title, config, mapping),
    onSuccess: (data, title) => {
      queryClient.setQueryData(keys.importTemplates(), data);
      onApplied(title);
      setName(null);
    },
  });
  const remove = useMutation({
    mutationFn: api.importTemplateDelete,
    onSuccess: (data) => {
      queryClient.setQueryData(keys.importTemplates(), data);
      onForget();
    },
  });
  const restore = useMutation({
    mutationFn: api.importPresetsRestore,
    onSuccess: (data) => queryClient.setQueryData(keys.importTemplates(), data),
  });

  const exists = templates.some((t) => t.name === name?.trim());

  return (
    <Buttons>
      <span className="dim">
        <Trans>Layout</Trans>
      </span>
      <Choice
        wide
        label={t`A ready-made layout for this broker`}
        placeholder={templates.length > 0 ? t`— as detected —` : t`— no layouts yet —`}
        value={applied}
        disabled={templates.length === 0}
        onChange={onApplied}
        // The user's own layouts come first, ungrouped; the shipped ones follow under their
        // heading, so a list of thirty brokers never buries the two the user made.
        options={templates.map((template) => ({
          value: template.name,
          label: template.name,
          group: template.source === "BUILTIN" ? t`Shipped` : undefined,
        }))}
      />
      <button
        className="btn btn--ghost btn--sm"
        title={t`Save the current layout as a template`}
        onClick={() => setName(applied)}
      >
        <FloppyDiskIcon /> <Trans>Save</Trans>
      </button>
      <button
        className="iconbtn iconbtn--sm iconbtn--danger"
        title={
          templates.find((t) => t.name === applied)?.source === "BUILTIN"
            ? t`Remove the shipped preset from the list`
            : t`Delete the selected layout`
        }
        disabled={!applied || remove.isPending}
        onClick={() => remove.mutate(applied)}
      >
        <TrashIcon />
      </button>
      <button
        className="iconbtn iconbtn--sm"
        title={t`Restore the removed shipped presets`}
        disabled={restore.isPending}
        onClick={() => restore.mutate()}
      >
        <ArrowCounterClockwiseIcon />
      </button>
      <ErrorText as="div" error={remove.error ?? restore.error} />

      {name !== null && (
        <FormDialog
          title={t`Save as a layout`}
          onClose={() => setName(null)}
          onSubmit={() => save.mutate(name.trim())}
          ready={Boolean(name.trim())}
          busy={save.isPending}
          error={save.error}
          submitLabel={exists ? t`Overwrite` : t`Save`}
        >
          <Field
            label={t`Layout name`}
            hint={t`It remembers the column layout and every alias. The next export from the same broker parses without questions.`}
          >
            <input
              autoFocus
              value={name}
              placeholder={t`For example: my broker`}
              onChange={(e) => setName(e.target.value)}
            />
          </Field>
        </FormDialog>
      )}
    </Buttons>
  );
}
