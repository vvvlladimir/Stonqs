import { Trans, useLingui } from "@lingui/react/macro";
import { useMemo, useState } from "react";
import { PencilSimpleIcon } from "@phosphor-icons/react";
import { Badge, Chip, Chips, DataTable, Panel, Scrolly, Tag } from "../../components/ui";
import { toNumber } from "../../lib/format";
import type { ImportField, ImportPreviewData, RowOverride, RowStatus } from "../../lib/types";
import { RowFix } from "./RowFix";
import { kindLabels, STATUS_LABELS, STATUS_TONES, type PreviewRow } from "./labels";

/** How many rows the review table renders before it stops being a review. */
const SHOWN = 200;

type Filter = RowStatus | "ALL" | "WARNING" | "FIXED";

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
  const { t, i18n } = useLingui();
  // Notices and hand edits are cross-cutting filters, not two more row statuses.
  const [filter, setFilter] = useState<Filter>("ALL");
  const [fixing, setFixing] = useState<PreviewRow | null>(null);

  const columns = Object.entries(preview.mapping.columns) as Array<[ImportField, string]>;
  const fixed = useMemo(() => new Set(overrides.map((o) => o.number)), [overrides]);

  const rows = useMemo(() => {
    if (filter === "ALL") return preview.rows;
    if (filter === "WARNING") {
      return preview.rows.filter((r) => r.problems.some((p) => p.severity === "WARNING"));
    }
    if (filter === "FIXED") return preview.rows.filter((r) => fixed.has(r.number));
    return preview.rows.filter((r) => r.status === filter);
  }, [preview.rows, filter, fixed]);

  const counts: Record<RowStatus, number> = {
    READY: preview.summary.ready,
    DUPLICATE: preview.summary.duplicates,
    UPDATED: preview.summary.updated,
    UNKNOWN_SECURITY: preview.summary.unknown_securities,
    IGNORED: preview.summary.ignored,
    INVALID: preview.summary.invalid,
  };

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
      <Chips>
        <Chip active={filter === "ALL"} onClick={() => setFilter("ALL")}>
          <Trans>all</Trans> · {preview.summary.total}
        </Chip>
        <Chip
          active={filter === "WARNING"}
          disabled={preview.summary.warnings === 0}
          onClick={() => setFilter("WARNING")}
        >
          <Trans>with notices</Trans> · {preview.summary.warnings}
        </Chip>
        {(Object.keys(STATUS_LABELS) as RowStatus[]).map((status) => (
          <Chip
            key={status}
            active={filter === status}
            disabled={counts[status] === 0}
            onClick={() => setFilter(status)}
          >
            {i18n._(STATUS_LABELS[status])} · {counts[status]}
          </Chip>
        ))}
        <Chip active={filter === "FIXED"} disabled={fixed.size === 0} onClick={() => setFilter("FIXED")}>
          <Trans>edited</Trans> · {fixed.size}
        </Chip>
      </Chips>

      <Scrolly x chain max={600}>
        <DataTable
          variant="rows"
          sizing="content"
          card={false}
          rows={rows.slice(0, SHOWN)}
          rowKey={(row: PreviewRow) => `${row.number}.${row.part}`}
          rowProps={(row: PreviewRow) => ({ onClick: () => setFixing(row) })}
          columns={[
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
          ]}
        />
      </Scrolly>

      {fixing && (
        <RowFix
          row={fixing}
          columns={columns}
          overrides={overrides}
          onChange={onOverrides}
          onClose={() => setFixing(null)}
        />
      )}
    </Panel>
  );
}
