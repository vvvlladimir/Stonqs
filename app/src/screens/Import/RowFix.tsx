import { Trans, useLingui } from "@lingui/react/macro";
import { useMemo, useState } from "react";
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
  // Fields the row needs and the file has no column for. A delivery stating only a quantity is
  // repaired by *adding* what it was worth, so those fields are offered even though no cell of
  // the file holds them; the core carries such an edit on its own (`RowInput::added`).
  const extra = useMemo(() => {
    const mapped = new Set(columns.map(([field]) => field));
    const wanted: ImportField[] = row.problems.some((p) => p.code === "DELIVERY_WITHOUT_COST")
      ? ["PRICE", "AMOUNT"]
      : [];
    return wanted.filter((field) => !mapped.has(field));
  }, [columns, row.problems]);

  const fields: Array<[ImportField, string | null]> = useMemo(
    () => [...columns.map(([field, column]) => [field, column] as [ImportField, string | null]),
           ...extra.map((field) => [field, null] as [ImportField, string | null])],
    [columns, extra],
  );

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
    for (const [field, column] of fields) {
      const was = column === null ? "" : (row.raw[column] ?? "");
      const value = draft[field] ?? "";
      if (value !== was) next.push({ number: row.number, field, value });
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
      {fields.map(([field, column]) => {
        const was = column === null ? "" : (row.raw[column] ?? "");
        const value = draft[field] ?? "";
        return (
          <Field
            key={field}
            label={fieldLabel(i18n, field)}
            // The original value is the thing a hand edit is judged against, so it stays visible.
            hint={
              column === null
                ? t`the file has no column for this — the value is added to the row`
                : value === was
                  ? t`column "${column}"`
                  : t`column "${column}" · was "${was}"`
            }
          >
            <input value={value} onChange={(e) => setDraft({ ...draft, [field]: e.target.value })} />
          </Field>
        );
      })}
    </FormDialog>
  );
}
