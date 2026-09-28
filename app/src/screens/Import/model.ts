/** What the wizard computes from a preview before anything is drawn: which rows a filter shows,
 *  which of them are worth an example, and how many the commit will actually write. Pure, so the
 *  number the user is asked to trust before pressing the button is testable on its own. */
import type { ImportOptions, ImportSummary, RowStatus } from "../../lib/types";
import type { PreviewRow } from "./labels";

/** Notices and hand edits are cross-cutting filters, not two more row statuses. */
export type RowFilter = RowStatus | "ALL" | "WARNING" | "FIXED";

export function filterRows(rows: PreviewRow[], filter: RowFilter, fixed: Set<number>): PreviewRow[] {
  if (filter === "ALL") return rows;
  if (filter === "WARNING") return rows.filter((r) => r.problems.some((p) => p.severity === "WARNING"));
  if (filter === "FIXED") return rows.filter((r) => fixed.has(r.number));
  return rows.filter((r) => r.status === filter);
}

/** Fields that are worth an example of their own: they are the ones that go wrong. */
export function shapeOf(row: PreviewRow): string {
  const d = row.draft;
  if (!d) return `—|${row.status}`;
  const has = [
    d.symbol || d.isin ? "sec" : "",
    Number(d.quantity) !== 0 ? "qty" : "",
    Number(d.price) !== 0 ? "price" : "",
    Number(d.amount) < 0 ? "neg" : "",
    Number(d.fees) !== 0 ? "fee" : "",
    Number(d.taxes) !== 0 ? "tax" : "",
    d.fx_rate_to_base ? "fx" : "",
    d.link_id ? "link" : "",
    d.note ? "note" : "",
  ]
    .filter(Boolean)
    .join("+");
  return `${d.kind}|${d.currency}|${has}|${row.status}`;
}

/** One row per distinct shape, in file order: the oddities are the question. */
export function distinctRows(rows: PreviewRow[]): PreviewRow[] {
  const seen = new Set<string>();
  return rows.filter((row) => {
    const shape = shapeOf(row);
    if (seen.has(shape)) return false;
    seen.add(shape);
    return true;
  });
}

/** How many rows the commit writes under these options — the figure on the button, so it is
 *  derived from the same summary the table is, never counted a second way. */
export function rowsToWrite(summary: ImportSummary, options: ImportOptions): number {
  return (
    summary.ready +
    summary.updated +
    (options.create_missing_securities ? summary.unknown_securities : 0) +
    (options.import_duplicates ? summary.duplicates : 0) +
    (options.import_similar ? summary.similar : 0)
  );
}
