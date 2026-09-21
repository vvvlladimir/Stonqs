import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { limitCurrency } from "../../lib/alerts";
import { ALERT_DIRECTIONS, ALERT_KINDS, alertDirectionLabel, alertKindLabel } from "../../lib/kinds";
import { affects, useInvalidate } from "../../lib/queries";
import { Field, FormDialog, Money } from "../ui";
import type { AlertInput, SecurityEventInput, SecurityRow } from "../../lib/types";

/**
 * The editor of one trigger. `securities` names the instrument and its current price; `fixed`
 * keeps the rule on the instrument it was opened for instead of offering a choice.
 */
export function AlertDialog({
  draft,
  securities,
  fixed,
  onChange,
  onClose,
}: {
  draft: AlertInput;
  securities: SecurityRow[];
  fixed?: boolean;
  onChange: (draft: AlertInput) => void;
  onClose: () => void;
}) {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const save = useMutation({
    mutationFn: api.alertSave,
    onSuccess: () => {
      invalidate(...affects.alerts);
      onClose();
    },
  });

  const security = securities.find((s) => s.id === draft.security_id);
  const price = draft.kind === "PRICE";
  const ready =
    draft.security_id !== "" &&
    (price ? Number(draft.price) > 0 && (draft.currency ?? "").trim() !== "" : Boolean(draft.date));

  return (
    <FormDialog
      title={draft.id ? t`Edit alert` : t`New alert`}
      onClose={onClose}
      onSubmit={() => save.mutate(draft)}
      busy={save.isPending}
      error={save.error}
      ready={ready}
    >
      {!fixed && (
        <Field
          label={t`Instrument`}
          options={securities.map((s) => ({ value: s.id, label: `${s.symbol} · ${s.name}` }))}
          value={draft.security_id}
          placeholder={t`— choose an instrument —`}
          onChange={(id) => {
            const chosen = securities.find((s) => s.id === id);
            onChange({
              ...draft,
              security_id: id,
              price: chosen?.last_close ?? draft.price,
              currency: limitCurrency(chosen),
            });
          }}
        />
      )}
      <Field
        label={t`Trigger`}
        options={ALERT_KINDS.map((kind) => ({ value: kind, label: alertKindLabel(i18n, kind) }))}
        value={draft.kind}
        onChange={(kind) => onChange({ ...draft, kind })}
      />
      {price && (
        <Field
          label={t`Direction`}
          options={ALERT_DIRECTIONS.map((d) => ({ value: d, label: alertDirectionLabel(i18n, d) }))}
          value={draft.direction ?? "BOTH"}
          onChange={(direction) => onChange({ ...draft, direction })}
        />
      )}
      {price ? (
        <Field
          label={t`Level`}
          hint={
            security?.last_close && security.quote_currency ? (
              <Trans>
                Now <Money value={security.last_close} currency={security.quote_currency} />. Every close that
                crosses the level in the chosen direction is logged and announced.
              </Trans>
            ) : (
              <Trans>
                Every close that crosses the level in the chosen direction is logged and announced.
              </Trans>
            )
          }
        >
          <div className="inline">
            <input
              inputMode="decimal"
              aria-label={t`Trigger level`}
              value={draft.price ?? ""}
              onChange={(e) => onChange({ ...draft, price: e.target.value })}
            />
            <input
              className="inp--pct"
              aria-label={t`Currency`}
              value={draft.currency ?? ""}
              onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
            />
          </div>
        </Field>
      ) : (
        <Field label={t`Date`} hint={t`Logged and announced once, on this day.`}>
          <input
            type="date"
            value={draft.date ?? ""}
            onChange={(e) => onChange({ ...draft, date: e.target.value })}
          />
        </Field>
      )}
      <Field label={t`Note`}>
        <input
          value={draft.note ?? ""}
          placeholder={t`optional`}
          onChange={(e) => onChange({ ...draft, note: e.target.value })}
        />
      </Field>
    </FormDialog>
  );
}

/**
 * The user's note on a date. On a dividend or split the provider reported, only the text is
 * editable: its date is the provider's.
 */
export function NoteDialog({
  draft,
  securities,
  reported,
  onChange,
  onClose,
}: {
  draft: SecurityEventInput;
  securities?: SecurityRow[];
  reported?: boolean;
  onChange: (draft: SecurityEventInput) => void;
  onClose: () => void;
}) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const save = useMutation({
    mutationFn: api.securityEventSave,
    onSuccess: () => {
      invalidate(...affects.alerts);
      onClose();
    },
  });

  const ready = draft.security_id !== "" && draft.date !== "" && (reported || draft.note.trim() !== "");

  return (
    <FormDialog
      title={draft.id ? t`Edit note` : t`New note`}
      onClose={onClose}
      onSubmit={() => save.mutate(draft)}
      busy={save.isPending}
      error={save.error}
      ready={ready}
    >
      {securities && (
        <Field
          label={t`Instrument`}
          options={securities.map((s) => ({ value: s.id, label: `${s.symbol} · ${s.name}` }))}
          value={draft.security_id}
          placeholder={t`— choose an instrument —`}
          onChange={(id) => onChange({ ...draft, security_id: id })}
        />
      )}
      <Field label={t`Date`}>
        <input
          type="date"
          value={draft.date}
          disabled={reported}
          onChange={(e) => onChange({ ...draft, date: e.target.value })}
        />
      </Field>
      <Field label={t`Note`}>
        <input value={draft.note} onChange={(e) => onChange({ ...draft, note: e.target.value })} />
      </Field>
    </FormDialog>
  );
}
