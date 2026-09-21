import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Banner, Field, FormDialog } from "../../components/ui";
import type { ImportField, RowOverride } from "../../lib/types";
import { fieldLabel, problemDetail, type PreviewRow } from "./labels";

/**
 * Repair one row by hand. Edits are collected and applied on save, so a row with
 * five wrong cells costs one recalculation rather than five.
 */
export function RowFix({
  row,
  columns,
  overrides,
  onChange,
  onClose,
}: {
  row: PreviewRow;
  columns: Array<[ImportField, string]>;
  overrides: RowOverride[];
  onChange: (next: RowOverride[]) => void;
  onClose: () => void;
}) {
  const { t, i18n } = useLingui();
  const [draft, setDraft] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      columns.map(([field, column]) => [
        field,
        overrides.find((o) => o.number === row.number && o.field === field)?.value ?? row.raw[column] ?? "",
      ]),
    ),
  );

  const edited = overrides.some((o) => o.number === row.number);

  const reset = () => {
    onChange(overrides.filter((o) => o.number !== row.number));
    onClose();
  };

  const save = () => {
    const next = overrides.filter((o) => o.number !== row.number);
    for (const [field, column] of columns) {
      if (draft[field] !== (row.raw[column] ?? "")) {
        next.push({ number: row.number, field, value: draft[field] });
      }
    }
    onChange(next);
    onClose();
  };

  return (
    <FormDialog
      title={t`Row ${row.number}`}
      onClose={onClose}
      onSubmit={save}
      submitLabel={t`Apply`}
      busyLabel={t`Applying…`}
      lead={
        edited ? (
          <button type="button" className="btn btn--ghost btn--danger" onClick={reset}>
            <Trans>Reset the edits</Trans>
          </button>
        ) : undefined
      }
    >
      {row.problems.map((problem, i) => (
        <Banner key={i} tone={problem.severity === "ERROR" ? "bad" : "warn"}>
          {problemDetail(i18n, problem)}
        </Banner>
      ))}
      {columns.map(([field, column]) => {
        const was = row.raw[column] ?? "";
        return (
          <Field
            key={field}
            label={fieldLabel(i18n, field)}
            // The original value is the thing a hand edit is judged against, so it stays visible.
            hint={draft[field] === was ? t`column "${column}"` : t`column "${column}" · was "${was}"`}
          >
            <input value={draft[field]} onChange={(e) => setDraft({ ...draft, [field]: e.target.value })} />
          </Field>
        );
      })}
    </FormDialog>
  );
}
