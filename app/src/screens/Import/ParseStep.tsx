import { Trans, useLingui } from "@lingui/react/macro";
import { useMemo } from "react";
import { Banner, Empty, Panel } from "../../components/ui";
import type {
  AccountRow,
  ImportMapping,
  ImportPreviewData,
  ImportTemplate,
  ParseConfig,
} from "../../lib/types";
import { FileTable, type FileTableSection } from "./FileTable";
import { ProblemList } from "./ProblemList";
import { ParsedPanel } from "./PreviewTables";
import { TargetSummary } from "./TargetSummary";
import { TemplateBar } from "./TemplateBar";
import { fieldLabel, missingFields, normalizeAlias, type PreviewRow } from "./labels";

/**
 * Teaching the app to read one broker's file. It is the same table as the step before —
 * the file's own columns and rows — narrowed to one row per distinct value of the file and
 * made editable in place: a header picks the field it carries, a cell picks what its value means.
 */
export function ParseStep({
  preview,
  mapping,
  config,
  accounts,
  templates,
  template,
  onTemplate,
  onForget,
  onChange,
  onConfig,
}: {
  preview: ImportPreviewData;
  mapping: ImportMapping;
  config: ParseConfig;
  accounts: AccountRow[];
  templates: ImportTemplate[];
  /** Name of the template this layout came from, or "" when it was made by hand. */
  template: string;
  onTemplate: (name: string) => void;
  /** Forgets the template's name without changing the layout it produced. */
  onForget: () => void;
  onChange: (m: ImportMapping) => void;
  onConfig: (c: ParseConfig, m: ImportMapping) => void;
}) {
  const { t, i18n } = useLingui();
  const missing = missingFields(mapping);
  // A template laid out for another broker points at columns this file does not have, and
  // then there is nothing left to map. Saying so is not enough: it has to be undoable.
  const stray = Object.values(mapping.columns).filter(
    (column): column is string => Boolean(column) && !preview.headers.includes(column),
  );
  const kindColumn = mapping.columns.KIND;
  const accountColumn = mapping.columns.ACCOUNT;
  // A skipped value is decided, not outstanding: it moves out of the work list.
  const todo = preview.kinds.filter((k) => !k.kind && !k.ignored);

  // The file's own row is the example: one per distinct value, found once and reused.
  const exampleOf = useMemo(() => {
    const by = new Map<string, PreviewRow>();
    if (!kindColumn) return by;
    for (const row of preview.rows) {
      const key = normalizeAlias(row.raw[kindColumn] ?? "");
      if (!by.has(key)) by.set(key, row);
    }
    return by;
  }, [preview.rows, kindColumn]);

  const accountExampleOf = useMemo(() => {
    const by = new Map<string, PreviewRow>();
    if (!accountColumn) return by;
    for (const row of preview.rows) {
      const key = normalizeAlias(row.raw[accountColumn] ?? "");
      if (!by.has(key)) by.set(key, row);
    }
    return by;
  }, [preview.rows, accountColumn]);

  const rowsFor = (values: string[]) =>
    values.map((value) => exampleOf.get(normalizeAlias(value))).filter(Boolean) as PreviewRow[];

  const sections: FileTableSection[] = [];

  if (!kindColumn) {
    sections.push({
      key: "rows",
      title: t`File rows`,
      note: t`point at the transaction-kind column in the header — only unique operations stay here then`,
      rows: preview.rows.slice(0, 20),
    });
  } else {
    const done = preview.kinds.filter((k) => k.kind || k.ignored);
    if (todo.length > 0) {
      sections.push({
        key: "kinds-todo",
        title: t`A choice is needed`,
        note: t`these transaction kinds were not recognized`,
        rows: rowsFor(todo.map((k) => k.value)),
      });
    }
    if (done.length > 0) {
      sections.push({
        key: "kinds-done",
        title: t`Recognized`,
        note: t`one row per unique transaction kind; "do not import" is a decision too`,
        rows: rowsFor(done.map((k) => k.value)),
      });
    }
  }

  if (accountColumn && preview.accounts.length > 0) {
    const rows = preview.accounts
      .map((a) => accountExampleOf.get(normalizeAlias(a.value)))
      .filter(Boolean) as PreviewRow[];
    if (rows.length > 0) {
      sections.push({
        key: "accounts",
        title: t`Accounts from the file`,
        note: t`one row per value of column "${accountColumn}"`,
        rows,
      });
    }
  }

  const total = sections.reduce((n, section) => n + section.rows.length, 0);

  return (
    <>
      {template && stray.length > 0 && (
        <Banner tone="bad">
          <Trans>
            Layout "{template}" does not belong to this file: it has no {stray.join(", ")} columns.
          </Trans>{" "}
          <button className="btn btn--sm" onClick={() => onTemplate("")}>
            <Trans>Restore what was detected</Trans>
          </button>
        </Banner>
      )}

      {missing.length > 0 && (
        <Banner tone="bad">
          <Trans>
            The file cannot be parsed without these columns:{" "}
            {missing.map((field) => fieldLabel(i18n, field)).join(", ")}. Point at them in the table header
            below.
          </Trans>
        </Banner>
      )}

      <TemplateBar
        config={config}
        mapping={mapping}
        templates={templates}
        applied={template}
        onApplied={onTemplate}
        onForget={onForget}
      />

      <TargetSummary
        preview={preview}
        mapping={mapping}
        config={config}
        accounts={accounts}
        onChange={onChange}
        onConfig={onConfig}
      />

      <Panel
        title={t`Mappings`}
        note={todo.length > 0 ? t`not mapped: ${todo.length}` : t`everything is mapped`}
        table
      >
        {total === 0 ? (
          <Empty title={t`Nothing to map`}>
            <Trans>The file holds no row with a transaction kind.</Trans>
          </Empty>
        ) : (
          <FileTable
            preview={preview}
            mapping={mapping}
            sections={sections}
            edit={{ onChange, accounts }}
            max={600}
          />
        )}
      </Panel>

      <ProblemList problems={preview.problems} />

      <ParsedPanel preview={preview} />
    </>
  );
}
