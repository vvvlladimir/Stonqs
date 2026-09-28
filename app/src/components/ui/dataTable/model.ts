import { currentLocale } from "../../../lib/i18n";
import type { SortValue, SortState, Column, Section, DataTableProps } from "./types";

export function used<T>(columns: DataTableProps<T>["columns"], isWide: boolean): Column<T>[] {
  return (columns.filter(Boolean) as Column<T>[]).filter((c) => c.only !== "wide" || isWide);
}

/** An empty head gets no classes: `acts` on a `<th>` would add a phantom column. */
export function headClasses<T>(column: Column<T>): string {
  const classes: string[] = [];
  if (column.align !== "left") classes.push("r", "num");
  if (column.className && column.header !== undefined) classes.push(column.className);
  return classes.join(" ");
}

export function cellClasses<T>(column: Column<T>, row?: T): string {
  const classes: string[] = [];
  if (column.align !== "left") classes.push("r", "num");
  if (column.className) classes.push(column.className);
  const extra = row !== undefined ? column.cellClass?.(row) : undefined;
  if (extra) classes.push(extra);
  return classes.join(" ");
}

/** Card slot of a column, defaulting to first = title, last = value, rest = facts. */
export function slotOf<T>(
  column: Column<T>,
  index: number,
  count: number,
): "title" | "value" | "fact" | "none" {
  if (column.card) return column.card;
  if (index === 0) return "title";
  if (index === count - 1) return "value";
  return "fact";
}

/** Rows are ordered inside each section: a sort re-reads a group, it never dissolves it. */
export function ordered<T>(
  sections: Section<T>[],
  columns: DataTableProps<T>["columns"],
  sort: SortState | null,
): Section<T>[] {
  if (!sort) return sections;
  const by = (columns.filter(Boolean) as Column<T>[]).find((c) => c.key === sort.key)?.sort;
  if (!by) return sections;
  const dir = sort.dir === "asc" ? 1 : -1;
  return sections.map((section) => ({
    ...section,
    rows: [...section.rows].sort((a, b) => compare(by(a), by(b), dir)),
  }));
}

/** Absent values sort last either way: "no price" is not "the lowest price". */
export function compare(a: SortValue, b: SortValue, dir: number): number {
  const missingA = a === null || a === undefined || a === "" || (typeof a === "number" && Number.isNaN(a));
  const missingB = b === null || b === undefined || b === "" || (typeof b === "number" && Number.isNaN(b));
  if (missingA || missingB) return missingA && missingB ? 0 : missingA ? 1 : -1;
  if (typeof a === "number" && typeof b === "number") return dir * (a - b);
  if (typeof a === "boolean" && typeof b === "boolean") return dir * (Number(a) - Number(b));
  // `numeric` so a column of strings that happen to be numbers ("10") still reads as numbers.
  return dir * String(a).localeCompare(String(b), currentLocale(), { numeric: true });
}
