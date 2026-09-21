import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { TrashIcon } from "@phosphor-icons/react";
import { api, today } from "../../lib/api";
import { affects, useCorporateActions, useInvalidate } from "../../lib/queries";
import { Async, DayMark, ErrorText, Field, FormDialog, List, ListRow, Num } from "../../components/ui";
import type { CorporateActionInput, SecurityRow } from "../../lib/types";

const BLANK = (date: string): CorporateActionInput => ({
  security_id: "",
  date,
  ratio_from: "1",
  ratio_to: "2",
});

/**
 * Splits of one instrument: the list, and one more at the bottom. Quotes arrive already
 * adjusted by the provider, so a split entered here moves lots and nothing else — which is
 * exactly why it has to be entered at all: a quantity from a broker export never is.
 */
export function Splits({ row, onClose }: { row: SecurityRow; onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const actions = useCorporateActions(row.id);
  const [draft, setDraft] = useState(BLANK(today()));

  const save = useMutation({
    mutationFn: api.corporateActionSave,
    onSuccess: () => {
      setDraft(BLANK(today()));
      invalidate(...affects.securities);
    },
  });
  const remove = useMutation({
    mutationFn: api.corporateActionDelete,
    onSuccess: () => invalidate(...affects.securities),
  });

  const positive = (value: string) => Number(value) > 0;
  const ready = draft.date !== "" && positive(draft.ratio_from) && positive(draft.ratio_to);

  return (
    <FormDialog
      title={t`Splits · ${row.symbol}`}
      onClose={onClose}
      onSubmit={() => save.mutate({ ...draft, security_id: row.id })}
      busy={save.isPending}
      error={save.error}
      ready={ready}
      submitLabel={t`Add split`}
      busyLabel={t`Adding…`}
    >
      <Async
        query={actions}
        isEmpty={(rows) => rows.length === 0}
        empty={
          <p className="muted">
            <Trans>No splits recorded for this instrument.</Trans>
          </p>
        }
      >
        {(rows) => (
          <List>
            {rows.map((action) => (
              <ListRow
                key={action.id}
                lead={<DayMark date={action.date} />}
                title={
                  <Trans>
                    <Num>{action.ratio_from}</Num> → <Num>{action.ratio_to}</Num>
                  </Trans>
                }
                sub={action.note ?? undefined}
                actions={
                  <button
                    type="button"
                    className="iconbtn iconbtn--sm iconbtn--danger"
                    aria-label={t`Delete the split`}
                    disabled={remove.isPending}
                    onClick={() => remove.mutate(action.id)}
                  >
                    <TrashIcon />
                  </button>
                }
              />
            ))}
          </List>
        )}
      </Async>
      <ErrorText error={remove.error} />

      <Field
        label={t`Ex-date`}
        hint={t`The new quantity applies from this day on, so a purchase made before it is restated and a later one is not.`}
      >
        <input
          type="date"
          value={draft.date}
          onChange={(e) => setDraft({ ...draft, date: e.target.value })}
        />
      </Field>
      <Field
        label={t`One old share becomes`}
        hint={t`A 2:1 split is 1 → 2; a 1:10 reverse split is 10 → 1. The position keeps its value: only the quantity and the per-share cost move.`}
      >
        <div className="inline">
          <input
            inputMode="decimal"
            className="inp--pct"
            aria-label={t`Old shares`}
            value={draft.ratio_from}
            onChange={(e) => setDraft({ ...draft, ratio_from: e.target.value })}
          />
          <span aria-hidden="true">→</span>
          <input
            inputMode="decimal"
            className="inp--pct"
            aria-label={t`New shares`}
            value={draft.ratio_to}
            onChange={(e) => setDraft({ ...draft, ratio_to: e.target.value })}
          />
        </div>
      </Field>
      <Field label={t`Note`}>
        <input
          value={draft.note ?? ""}
          placeholder={t`optional`}
          onChange={(e) => setDraft({ ...draft, note: e.target.value })}
        />
      </Field>
    </FormDialog>
  );
}
