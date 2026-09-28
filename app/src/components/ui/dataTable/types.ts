import type { HTMLAttributes, ReactNode } from "react";

export type SortDir = "asc" | "desc";

/** What a row is compared by: never the rendered cell, which may be a whole component. */
export type SortValue = string | number | boolean | null | undefined;

export interface SortState {
  key: string;
  dir: SortDir;
}

export interface Column<T> {
  key: string;
  header?: ReactNode;
  cell: (row: T) => ReactNode;
  /** Numeric columns are right-aligned; `left` opts out (logos, sparklines). */
  align?: "left" | "right";
  /** Column width, declared here rather than as a per-screen CSS rule. */
  width?: string;
  /** Dropped entirely below the wide breakpoint, where the row has no space for it. */
  only?: "wide";
  /** Extra class on the cell, per row (sign colouring of a whole cell). */
  cellClass?: (row: T) => string | undefined;
  /** Class on both header and cells. */
  className?: string;
  /** `false` keeps `sizing="content"` from clamping the cell — it holds a control, not text. */
  clamp?: false;
  /** Slot in the card layout; by default the first column titles the card,
   *  the last one carries its value and the rest become facts. */
  card?: "title" | "value" | "fact" | "none";
  /** Header text repeated as the fact's label on cards. */
  factLabel?: ReactNode;
  /** Totals cell; a footer row appears as soon as one column declares one. */
  foot?: ReactNode;
  /** The value rows are ordered by; a cell is markup, so it is declared, not read off. */
  sort?: (row: T) => SortValue;
  /** Direction the first click picks; text opens at A, every other column at the largest. */
  sortFirst?: SortDir;
  /** Explains a heading that does not speak for itself: the column's accessible name,
   *  and the tooltip the heading shows on hover. */
  ariaLabel?: string;
}

/** A run of rows under one or more sticky group headings. */
export interface Section<T> {
  key: string;
  rows: T[];
  /** Group rows opening the section in table mode, outermost first; the caller
   *  supplies the cells, because a group line rarely spans the whole width. */
  heads?: Array<{ key: string; className?: string; cells: ReactNode }>;
  /** Heading above the section's cards in the narrow layout. */
  cardHead?: ReactNode;
}

export interface DataTableProps<T> {
  /** `false` entries are dropped, so a column can be toggled inline. */
  columns: Array<Column<T> | false | null | undefined>;
  rows?: T[];
  /** Grouped alternative to `rows`; both are never given at once. */
  sections?: Array<Section<T>>;
  rowKey: (row: T) => string;
  rowProps?: (row: T) => HTMLAttributes<HTMLElement>;
  /** Rows are clickable (`tbl--rows`) or nested inside another table. */
  variant?: "rows" | "nested";
  /** Column widths only hold with a fixed layout. */
  fixed?: boolean;
  /** Columns sized by content; the table overflows and must sit in a `Scrolly x`. */
  sizing?: "content";
  className?: string;
  /** Extra footer rows, wrapped in their own `<tr>`s by the caller. */
  foot?: ReactNode;
  /** Narrow layout: derived cards, a screen's own presenter, or `false` to keep the table. */
  card?: false | ((row: T) => ReactNode);
  /** Card list laid out as plain lines rather than boxes. */
  cards?: "cards" | "lines";
  /** Shown when there is nothing to render at all. */
  empty?: ReactNode;
  /** Ordering the table opens with; clicking a heading takes it from there. */
  defaultSort?: SortState;
  /** Caller-held ordering, with `onSortChange`; otherwise the table keeps its own. */
  sort?: SortState | null;
  onSortChange?: (sort: SortState | null) => void;
}
