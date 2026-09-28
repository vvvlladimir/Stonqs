import type { ReactNode } from "react";
import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { SortValue } from "../../ui";
import type {
  CostBasisMethod,
  PositionCostRow,
  PositionReturnRow,
  PositionRow,
  SecurityRow,
} from "../../../lib/types";

/** Column groups by what a figure is read off, which is what tells similar names apart. */
export const GROUPS = [
  { id: "quotes", label: msg`Price` },
  { id: "levels", label: msg`Levels` },
  { id: "value", label: msg`Value and cost` },
  { id: "result", label: msg`Result` },
  { id: "income", label: msg`Dividends` },
  { id: "costs", label: msg`Costs` },
  { id: "risk", label: msg`Risk` },
  { id: "holding", label: msg`Your position` },
  { id: "record", label: msg`Instrument` },
  { id: "attrs", label: msg`Attributes` },
] as const;

export type GroupId = (typeof GROUPS)[number]["id"];

/** A column of a table whose rows are `Row` and whose cells may read `Ctx`. */
export interface ColumnOf<Row, Ctx> {
  id: string;
  /** Where the picker files it. */
  group: GroupId;
  label: (i18n: I18n, currency: string) => string;
  /** One short sentence saying what the figure is; the heading's tooltip and its aria name. */
  tip?: (i18n: I18n) => string;
  cell: (row: Row, ctx: Ctx) => ReactNode;
  /** What the column is ordered by; a column without one is not sortable (a sparkline). */
  sort?: (row: Row, ctx: Ctx) => SortValue;
  /** Color by sign when a signed value is available. */
  tone?: (row: Row, ctx: Ctx) => string;
  /** Right-align numbers, but not images. */
  align?: "left";
  /** Width the figure needs; the instrument column lives on what is left over. */
  width: string;
}

/** What a cell may read besides the position itself. */
export interface CellCtx {
  currency: string;
  /** Cells that render a label table resolve it here, at render time. */
  i18n: I18n;
  /** The position's return over the chosen period; absent while that query is loading. */
  period?: PositionReturnRow;
  /** The instrument's directory record: identifiers, note and the user's own attributes. */
  security?: SecurityRow;
  /** The same position under both cost-basis methods; absent while that query is idle. */
  cost?: PositionCostRow;
  /** The method the portfolio itself is kept under: that one column is already in the row. */
  own?: CostBasisMethod;
}

/** A column the user can switch off; the instrument and the menu are always shown. */
export type Column = ColumnOf<PositionRow, CellCtx>;

/** What a cell needs to render the instrument's own record, whatever table it sits in. */
export interface RecordCtx {
  i18n: I18n;
  security?: SecurityRow;
}
