import { useAttributeDefs } from "../../lib/queries";
import type { PositionRow } from "../../lib/types";
import { COLUMNS } from "./positionColumns/columns";
import { attributeColumns } from "./positionColumns/record";
import type { CellCtx, Column } from "./positionColumns/types";

export { COLUMNS } from "./positionColumns/columns";
export { GROUPS } from "./positionColumns/types";
export type { CellCtx, Column, ColumnOf, GroupId, RecordCtx } from "./positionColumns/types";
export { dash, periodCell } from "./positionColumns/cells";
export { needsCostQuery, resolveColumnIds } from "./positionColumns/cost";
export { attributeColumns, recordColumns } from "./positionColumns/record";

/** Every column the table can offer: the fixed figures, then the user's own attributes. */
export function useAllColumns(): Column[] {
  const defs = useAttributeDefs();
  return [...COLUMNS, ...attributeColumns<PositionRow, CellCtx>(defs.data ?? [])];
}

/** The chosen columns in the order they were chosen; an id no catalogue entry answers is dropped. */
export function orderColumns<C extends { id: string }>(all: C[], ids: string[]): C[] {
  const byId = new Map(all.map((column) => [column.id, column]));
  return ids.map((id) => byId.get(id)).filter((column): column is C => column !== undefined);
}
