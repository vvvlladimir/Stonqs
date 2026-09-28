import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useMemo, useState } from "react";
import { Badge, Buttons, DataTable, Panel, Scrolly } from "../../components/ui";
import { toNumber } from "../../lib/format";
import type { ImportPreviewData } from "../../lib/types";
import { kindLabels, STATUS_LABELS, STATUS_TONES, type PreviewRow } from "./labels";
import { distinctRows } from "./model";

export function ParsedPanel({ preview }: { preview: ImportPreviewData }) {
  const { t } = useLingui();
  const [all, setAll] = useState(false);

  const unique = useMemo(() => distinctRows(preview.rows), [preview.rows]);
  const rows = all ? preview.rows : unique;

  return (
    <Panel
      title={t`What it becomes`}
      note={
        all
          ? plural(preview.rows.length, { one: "# row", other: "# rows" })
          : t`${plural(unique.length, { one: "# unique transaction", other: "# unique transactions" })} of ${preview.rows.length}`
      }
      tools={
        <Buttons>
          <button className="iconbtn iconbtn--sm" onClick={() => setAll(!all)}>
            {all ? t`unique only` : t`show all ${preview.rows.length}`}
          </button>
        </Buttons>
      }
      table
    >
      <ParsedTable rows={rows} />
    </Panel>
  );
}

function ParsedTable({ rows }: { rows: PreviewRow[] }) {
  const { t, i18n } = useLingui();
  return (
    <Scrolly x chain max={600}>
      <DataTable
        variant="nested"
        sizing="content"
        card={false}
        rows={rows}
        rowKey={(row) => String(row.number)}
        columns={[
          {
            key: "#",
            sort: (row) => row.number,
            header: "#",
            align: "left",
            className: "dim",
            cell: (row) => row.number,
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
            key: "price",
            sort: (row) => toNumber(row.draft?.price),
            header: t`Price`,
            cell: (row) => row.draft?.price ?? "—",
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
            key: "fx",
            sort: (row) => toNumber(row.draft?.fx_rate_to_base),
            header: t`FX rate`,
            cell: (row) => row.draft?.fx_rate_to_base ?? "—",
          },
          {
            key: "link",
            sort: (row) => row.draft?.link_id,
            header: t`Link`,
            align: "left",
            cell: (row) => row.draft?.link_id ?? "—",
          },
          {
            key: "note",
            sort: (row) => row.draft?.note,
            header: t`Note`,
            align: "left",
            cell: (row) => row.draft?.note ?? "—",
          },
          {
            key: "status",
            sort: (row) => i18n._(STATUS_LABELS[row.status]),
            header: t`Status`,
            align: "left",
            clamp: false,
            cell: (row) => <Badge tone={STATUS_TONES[row.status]}>{i18n._(STATUS_LABELS[row.status])}</Badge>,
          },
        ]}
      />
    </Scrolly>
  );
}
