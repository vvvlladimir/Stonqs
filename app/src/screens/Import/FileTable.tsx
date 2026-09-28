import { useLingui } from "@lingui/react/macro";
import type { ReactNode } from "react";
import { Choice, DataTable, Scrolly, Tag } from "../../components/ui";
import { accountKindLabel } from "../../lib/kinds";
import type { AccountRow, ImportField, ImportMapping, ImportPreviewData } from "../../lib/types";
import {
  FIELDS,
  kinds,
  REQUIRED_FIELDS,
  SKIP,
  assignAccounts,
  assignColumn,
  assignKinds,
  fieldByColumn,
  fieldLabel,
  normalizeAlias,
  SPLITS,
  splitOf,
  type KindChoice,
  type PreviewRow,
} from "./labels";

/** Reading and mapping are one table: a mapping is judged against the rows it changes. */

export interface FileTableEdit {
  onChange: (mapping: ImportMapping) => void;
  accounts: AccountRow[];
}

export interface FileTableSection {
  key: string;
  title: ReactNode;
  note?: ReactNode;
  rows: PreviewRow[];
}

export function FileTable({
  preview,
  mapping,
  rows,
  sections,
  edit,
  max = 460,
}: {
  preview: ImportPreviewData;
  mapping: ImportMapping;
  /** Flat alternative to `sections`; both are never given at once. */
  rows?: PreviewRow[];
  sections?: FileTableSection[];
  /** Absent for a read-only preview: the headers then only state what was detected. */
  edit?: FileTableEdit;
  max?: number;
}) {
  const { t, i18n } = useLingui();
  const field = fieldByColumn(mapping);
  const kindColumn = mapping.columns.KIND;
  const accountColumn = mapping.columns.ACCOUNT;

  const kindOf = new Map(preview.kinds.map((k) => [normalizeAlias(k.value), k.kind]));
  const skipped = new Set(preview.kinds.filter((k) => k.ignored).map((k) => normalizeAlias(k.value)));
  const accountOf = new Map(preview.accounts.map((a) => [normalizeAlias(a.value), a.account_id]));

  const fieldOptions = FIELDS.map(([id, title]) => ({
    value: id,
    label: REQUIRED_FIELDS.includes(id) ? `${i18n._(title)} *` : i18n._(title),
  }));
  const kindOptions = [
    ...kinds(i18n).map(([id, title]) => ({ value: id, label: title })),
    ...Object.entries(SPLITS).map(([id, split]) => ({ value: id, label: i18n._(split.label) })),
    { value: SKIP, label: t`do not import` },
  ];
  const accountOptions = (edit?.accounts ?? []).map((account) => ({
    value: account.id,
    label: `${accountKindLabel(i18n, account.kind)} · ${account.name} · ${account.currency}`,
  }));

  /** Mapped columns lead, unmapped ones step back: the eye should land on the work. */
  const tone = (name: string) => (field[name] ? "mono" : "mono dim");

  const kindCell = (row: PreviewRow) => {
    const value = row.raw[kindColumn!] ?? "";
    const key = normalizeAlias(value);
    const kind = kindOf.get(key) ?? null;
    const skip = skipped.has(key);
    // A wording answered by a rule shows that answer, not the kind it happens to alias.
    const split = splitOf(mapping, value);
    return (
      <Pair value={value} done={Boolean(kind) || skip || Boolean(split)}>
        <Choice
          wide
          label={t`Transaction kind for "${value}"`}
          placeholder={t`— not mapped —`}
          value={split ?? (skip ? SKIP : (kind ?? ""))}
          onChange={(next) => edit!.onChange(assignKinds(mapping, [value], next as KindChoice))}
          options={kindOptions}
        />
      </Pair>
    );
  };

  const accountCell = (row: PreviewRow) => {
    const value = row.raw[accountColumn!] ?? "";
    const id = accountOf.get(normalizeAlias(value)) ?? null;
    return (
      <Pair value={value} done={Boolean(id)}>
        <Choice
          wide
          label={t`Account for "${value}"`}
          placeholder={t`— choose an account —`}
          value={id ?? ""}
          onChange={(next) => edit!.onChange(assignAccounts(mapping, [value], next))}
          options={accountOptions}
        />
      </Pair>
    );
  };

  const columns = [
    {
      key: "#",
      header: "#",
      align: "left" as const,
      className: "dim",
      cell: (row: PreviewRow) => row.number,
    },
    ...preview.headers.map((name) => {
      const editable = edit && (name === kindColumn || name === accountColumn);
      return {
        key: name,
        header: (
          <ColumnHead name={name} field={field[name]} mapping={mapping} edit={edit} options={fieldOptions} />
        ),
        align: "left" as const,
        className: editable ? "ask" : tone(name),
        clamp: editable ? (false as const) : undefined,
        cell: (row: PreviewRow) =>
          name === kindColumn && edit
            ? kindCell(row)
            : name === accountColumn && edit
              ? accountCell(row)
              : row.raw[name] || "—",
      };
    }),
  ];

  return (
    <Scrolly x chain max={max}>
      <DataTable
        variant="nested"
        sizing="content"
        card={false}
        rows={sections ? undefined : (rows ?? [])}
        sections={sections?.map((section) => ({
          key: section.key,
          rows: section.rows,
          heads: [
            {
              key: "head",
              cells: (
                <td colSpan={columns.length}>
                  {section.title}
                  {section.note !== undefined && <span className="dim"> · {section.note}</span>}
                </td>
              ),
            },
          ],
        }))}
        rowKey={(row: PreviewRow) => String(row.number)}
        columns={columns}
      />
    </Scrolly>
  );
}

/** "their value = our value", the one sentence every decision here is written in. */
function Pair({ value, done, children }: { value: string; done: boolean; children: ReactNode }) {
  const { t } = useLingui();
  return (
    <>
      <div className="inline">
        <b className="mono">{value || "—"}</b>
        {!done && <Tag warn>{t`not mapped`}</Tag>}
      </div>
      <div className="inline">
        <span className="dim">=</span>
        {children}
      </div>
    </>
  );
}

/** A file column's name over the field it is read as: a choice while editing, a statement otherwise. */
function ColumnHead({
  name,
  field,
  mapping,
  edit,
  options,
}: {
  name: string;
  field: ImportField | undefined;
  mapping: ImportMapping;
  edit: FileTableEdit | undefined;
  options: Array<{ value: ImportField; label: string }>;
}) {
  const { t, i18n } = useLingui();
  return (
    <>
      <div className={field ? "nm" : "dim"}>{name}</div>
      {edit ? (
        <Choice
          tight
          label={t`Field for column "${name}"`}
          placeholder={t`— not used —`}
          value={field ?? ""}
          onChange={(next) => edit.onChange(assignColumn(mapping, name, next as ImportField | ""))}
          options={options}
        />
      ) : (
        <div className="sub">{field ? fieldLabel(i18n, field) : t`not used`}</div>
      )}
    </>
  );
}
