import { useState } from "react";
import { ArrowCounterClockwiseIcon, PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";
import { msg } from "@lingui/core/macro";
import type { I18n } from "@lingui/core";
import {
  BUILTIN_LABELS,
  BUILTIN_PRESETS,
  DEFAULT_SPEC,
  UNIT_LABELS,
  newPeriodId,
  usePeriodEdit,
  usePeriodSettings,
} from "../../lib/periods";
import { Async, ErrorText, Field, FormDialog, List, ListRow, Modal } from "../ui";
import { formatDay } from "../../lib/format";
import type { PeriodSpec, PeriodUnit, UserPeriod } from "../../lib/types";

/** The open end of a fixed window, written out where its dates are listed. */
const TODAY = msg`today`;

/**
 * The period axis, edited in one place. A period the user adds is theirs to delete; a shipped
 * preset is only hidden, so "restore" can bring the seven back — the same bargain the shipped
 * import layouts make.
 */
export function PeriodEditor({ onClose }: { onClose: () => void }) {
  const { t, i18n } = useLingui();
  const settings = usePeriodSettings();
  const { remove, restore } = usePeriodEdit();
  const [draft, setDraft] = useState<UserPeriod | null>(null);

  const hidden = settings.data?.hidden_presets ?? [];

  return (
    <Modal
      title={t`Periods`}
      onClose={onClose}
      foot={
        <>
          <button
            type="button"
            className="btn btn--ghost"
            disabled={hidden.length === 0 || restore.isPending}
            title={t`Bring back the shipped periods that were removed`}
            onClick={() => restore.mutate()}
          >
            <ArrowCounterClockwiseIcon /> <Trans>Restore</Trans>
          </button>
          <span className="spacer" />
          <button
            type="button"
            className="btn"
            onClick={() => setDraft({ id: newPeriodId(), name: "", spec: DEFAULT_SPEC })}
          >
            <PlusIcon /> <Trans>New period</Trans>
          </button>
        </>
      }
    >
      <ErrorText error={remove.error ?? restore.error} />
      <Async query={settings}>
        {(data) => (
          <List as="ul">
            {BUILTIN_PRESETS.filter((id) => !data.hidden_presets.includes(id)).map((id) => (
              <ListRow
                key={id}
                as="li"
                title={i18n._(BUILTIN_LABELS[id])}
                sub={t`Shipped with the app`}
                end={
                  <button
                    type="button"
                    className="iconbtn"
                    aria-label={t`Remove from the period strip`}
                    title={t`Remove from the period strip`}
                    disabled={remove.isPending}
                    onClick={() => remove.mutate(id)}
                  >
                    <TrashIcon />
                  </button>
                }
              />
            ))}
            {data.periods.map((period) => (
              <ListRow
                key={period.id}
                as="li"
                onClick={() => setDraft(period)}
                title={period.name}
                sub={describe(period.spec, i18n)}
                end={
                  <button
                    type="button"
                    className="iconbtn"
                    aria-label={t`Delete this period`}
                    title={t`Delete this period`}
                    disabled={remove.isPending}
                    onClick={(e) => {
                      e.stopPropagation();
                      remove.mutate(period.id);
                    }}
                  >
                    <TrashIcon />
                  </button>
                }
              />
            ))}
          </List>
        )}
      </Async>

      {draft && <PeriodForm draft={draft} onChange={setDraft} onClose={() => setDraft(null)} />}
    </Modal>
  );
}

function PeriodForm({
  draft,
  onChange,
  onClose,
}: {
  draft: UserPeriod;
  onChange: (period: UserPeriod) => void;
  onClose: () => void;
}) {
  const { t, i18n } = useLingui();
  const { save } = usePeriodEdit();
  const { spec } = draft;

  // A fixed window must not end before it starts; the host refuses one too, but saying so
  // here is the difference between a hint and a failed round trip. An empty end is not a
  // broken one — it means the window runs to today.
  const ordered = spec.kind !== "FIXED" || spec.to === null || spec.from <= spec.to;
  const filled = spec.kind === "RELATIVE" ? spec.count > 0 : spec.from !== "";
  const ready = draft.name.trim() !== "" && ordered && filled;

  return (
    <FormDialog
      title={t`Period`}
      onClose={onClose}
      busy={save.isPending}
      error={save.error}
      ready={ready}
      onSubmit={() => save.mutate(draft, { onSuccess: onClose })}
    >
      <Field label={t`Name`}>
        <input
          value={draft.name}
          autoFocus
          placeholder={t`Six months`}
          onChange={(e) => onChange({ ...draft, name: e.target.value })}
        />
      </Field>

      <Field<PeriodSpec["kind"]>
        label={t`Kind`}
        value={spec.kind}
        options={[
          { value: "RELATIVE", label: t`A window back from today` },
          { value: "FIXED", label: t`Two dates` },
        ]}
        onChange={(kind) =>
          onChange({
            ...draft,
            spec: kind === "RELATIVE" ? DEFAULT_SPEC : { kind: "FIXED", from: "", to: null },
          })
        }
        hint={
          spec.kind === "RELATIVE"
            ? t`Moves with the calendar: tomorrow it covers one more day.`
            : t`Stays where it is put. A window that ends in the future stops at today.`
        }
      />

      {spec.kind === "RELATIVE" ? (
        <>
          <Field label={t`Length`}>
            <input
              type="number"
              min={1}
              value={spec.count}
              onChange={(e) => onChange({ ...draft, spec: { ...spec, count: Number(e.target.value) } })}
            />
          </Field>
          <Field<PeriodUnit>
            label={t`Unit`}
            value={spec.unit}
            options={(Object.keys(UNIT_LABELS) as PeriodUnit[]).map((unit) => ({
              value: unit,
              label: i18n._(UNIT_LABELS[unit]),
            }))}
            onChange={(unit) => onChange({ ...draft, spec: { ...spec, unit } })}
          />
        </>
      ) : (
        <>
          <Field label={t`From`}>
            <input
              type="date"
              value={spec.from}
              onChange={(e) => onChange({ ...draft, spec: { ...spec, from: e.target.value } })}
            />
          </Field>
          <Field
            label={t`To`}
            hint={ordered ? t`Leave empty to run up to today.` : t`The period ends before it starts.`}
          >
            <input
              type="date"
              value={spec.to ?? ""}
              min={spec.from || undefined}
              onChange={(e) => onChange({ ...draft, spec: { ...spec, to: e.target.value || null } })}
            />
          </Field>
        </>
      )}
    </FormDialog>
  );
}

/** One line describing what a period resolves to, shown under its name. */
function describe(spec: PeriodSpec, i18n: I18n): string {
  if (spec.kind !== "FIXED") return `${spec.count} ${i18n._(UNIT_LABELS[spec.unit])}`;
  const end = spec.to ? formatDay(spec.to) : i18n._(TODAY);
  return `${formatDay(spec.from)} — ${end}`;
}
