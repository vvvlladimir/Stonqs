import { Trans, useLingui } from "@lingui/react/macro";
import { useMemo, useState } from "react";
import { PencilSimpleIcon } from "@phosphor-icons/react";
import { Badge, Chip, Chips, DataTable, Panel, Scrolly, Tag, type Column } from "../../components/ui";
import { toNumber } from "../../lib/format";
import type { ImportField, ImportPreviewData, RowOverride, RowStatus } from "../../lib/types";
import { RowFix } from "./RowFix";
import { kindLabels, STATUS_LABELS, STATUS_TONES, type PreviewRow } from "./labels";
import { filterRows, type RowFilter } from "./model";

/** How many rows the review table renders before it stops being a review. */
const SHOWN = 200;

/** Every row of the file, filtered by what the user is looking for and fixable in place. */
export function RowTable({
  preview,
  overrides,
  onOverrides,
}: {
  preview: ImportPreviewData;
  overrides: RowOverride[];
  onOverrides: (next: RowOverride[]) => void;
}) {
  const { t } = useLingui();
  // Notices and hand edits are cross-cutting filters, not two more row statuses.
  const [filter, setFilter] = useState<RowFilter>("ALL");
  const [fixing, setFixing] = useState<PreviewRow | null>(null);

  const fileColumns = Object.entries(preview.mapping.columns) as Array<[ImportField, string]>;
  const fixed = useMemo(() => new Set(overrides.map((o) => o.number)), [overrides]);

  const columns = useRowColumns(fixed);
  const rows = useMemo(() => filterRows(preview.rows, filter, fixed), [preview.rows, filter, fixed]);

  return (
    <Panel
      title={t`File rows`}
      note={
        rows.length > SHOWN
          ? t`first ${SHOWN} of ${rows.length}`
          : t`${rows.length} of ${preview.rows.length}`
      }
      tools={
        <span className="dim">
          <Trans>a row opens for editing</Trans>
        </span>
      }
      table
    >
      <StatusChips summary={preview.summary} filter={filter} onFilter={setFilter} edited={fixed.size} />

      <Scrolly x chain max={600}>
        <DataTable
          variant="rows"
          sizing="content"
          card={false}
          rows={rows.slice(0, SHOWN)}
          rowKey={(row: PreviewRow) => `${row.number}.${row.part}`}
          rowProps={(row: PreviewRow) => ({ onClick: () => setFixing(row) })}
          columns={columns}
        />
      </Scrolly>

      {fixing && (
        <RowFix
          row={fixing}
          columns={fileColumns}
          overrides={overrides}
          onChange={onOverrides}
          onClose={() => setFixing(null)}
        />
      )}
    </Panel>
  );
}

/** One chip per filter, each with how many rows it would show; an empty one cannot be picked. */
function StatusChips({
  summary,
  filter,
  onFilter,
  edited,
}: {
  summary: ImportPreviewData["summary"];
  filter: RowFilter;
  onFilter: (filter: RowFilter) => void;
  edited: number;
}) {
  const { i18n } = useLingui();
  const counts: Record<RowStatus, number> = {
    READY: summary.ready,
    DUPLICATE: summary.duplicates,
    UPDATED: summary.updated,
    SIMILAR: summary.similar,
    UNKNOWN_SECURITY: summary.unknown_securities,
    IGNORED: summary.ignored,
    INVALID: summary.invalid,
  };
  return (
    <Chips>
      <Chip active={filter === "ALL"} onClick={() => onFilter("ALL")}>
        <Trans>all</Trans> · {summary.total}
      </Chip>
      <Chip
        active={filter === "WARNING"}
        disabled={summary.warnings === 0}
        onClick={() => onFilter("WARNING")}
      >
        <Trans>with notices</Trans> · {summary.warnings}
      </Chip>
      {(Object.keys(STATUS_LABELS) as RowStatus[]).map((status) => (
        <Chip
          key={status}
          active={filter === status}
          disabled={counts[status] === 0}
          onClick={() => onFilter(status)}
        >
          {i18n._(STATUS_LABELS[status])} · {counts[status]}
        </Chip>
      ))}
      <Chip active={filter === "FIXED"} disabled={edited === 0} onClick={() => onFilter("FIXED")}>
        <Trans>edited</Trans> · {edited}
      </Chip>
    </Chips>
  );
}

/** The file's rows as they will be written, with the first notice each carries. */
function useRowColumns(fixed: Set<number>): Array<Column<PreviewRow>> {
  const { t, i18n } = useLingui();
  return [
    {
      key: "#",
      sort: (row) => row.number,
      header: "№",
      align: "left",
      className: "dim",
      // A rule can turn one file line into several operations; they share its number.
      cell: (row) => (row.part > 1 ? `${row.number}.${row.part}` : row.number),
    },
    {
      key: "status",
      sort: (row) => i18n._(STATUS_LABELS[row.status]),
      header: t`Status`,
      align: "left",
      clamp: false,
      cell: (row) => (
        <span className="inline">
          <Badge tone={STATUS_TONES[row.status]}>{i18n._(STATUS_LABELS[row.status])}</Badge>
          {fixed.has(row.number) && <Tag>{t`edited`}</Tag>}
        </span>
      ),
    },
    {
      key: "date",
      sort: (row) => row.draft?.date,
      header: t`Date`,
      align: "left",
      cell: (row) => row.draft?.date ?? "—",
    },
    {
      key: "kind",
      sort: (row) => (row.draft ? (kindLabels(i18n)[row.draft.kind] ?? row.draft.kind) : null),
      header: t`Kind`,
      align: "left",
      cell: (row) => (row.draft ? (kindLabels(i18n)[row.draft.kind] ?? row.draft.kind) : "—"),
    },
    {
      key: "symbol",
      sort: (row) => row.draft?.symbol || row.draft?.isin,
      header: t`Instrument`,
      align: "left",
      cell: (row) => row.draft?.symbol || row.draft?.isin || "—",
    },
    {
      key: "quantity",
      sort: (row) => toNumber(row.draft?.quantity),
      header: t`Qty`,
      cell: (row) => row.draft?.quantity ?? "—",
    },
    {
      key: "amount",
      sort: (row) => toNumber(row.draft?.amount),
      header: t`Amount`,
      cell: (row) => row.draft?.amount ?? "—",
    },
    {
      key: "currency",
      sort: (row) => row.draft?.currency,
      header: t`Currency`,
      align: "left",
      cell: (row) => row.draft?.currency ?? "—",
    },
    {
      key: "fees",
      sort: (row) => toNumber(row.draft?.fees),
      header: t`Commission`,
      cell: (row) => row.draft?.fees ?? "—",
    },
    {
      key: "taxes",
      sort: (row) => toNumber(row.draft?.taxes),
      header: t`Tax`,
      cell: (row) => row.draft?.taxes ?? "—",
    },
    {
      key: "note",
      sort: (row) => row.problems.length,
      header: t`Notice`,
      align: "left",
      clamp: false,
      cell: (row) =>
        row.problems.length > 0 ? (
          <span className="inline">
            <Tag warn>{row.problems[0].message}</Tag>
            {row.problems.length > 1 && <Badge>+{row.problems.length - 1}</Badge>}
          </span>
        ) : (
          <span className="dim">—</span>
        ),
    },
    {
      key: "fix",
      header: "",
      align: "left",
      width: "32px",
      clamp: false,
      cell: () => <PencilSimpleIcon className="dim" />,
    },
  ];
}
