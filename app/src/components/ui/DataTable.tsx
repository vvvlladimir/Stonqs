import { CaretDownIcon, CaretUpDownIcon, CaretUpIcon } from "@phosphor-icons/react";
import { Fragment, useEffect, useRef, useState, type HTMLAttributes, type ReactNode } from "react";
import { currentLocale } from "../../lib/i18n";
import { useIsWide } from "../../lib/useLayout";
import { List, ListRow } from "./List";

/**
 * The only `<table>` in the product. A screen describes its columns; the primitive
 * decides markup, alignment, widths, ordering and — below the wide breakpoint — whether
 * the same columns are shown as cards instead.
 */

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
  /**
   * Makes the column sortable: the value its rows are ordered by. Declared rather than
   * read off the cell, because a cell is markup — a logo, a bar, a whole component.
   */
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
  /**
   * `content` sizes every column by what is in it instead of sharing the width out:
   * the table grows past its container and is expected to sit inside a `Scrolly x`.
   * Cells are capped, wrapping to two lines before they are cut.
   */
  sizing?: "content";
  className?: string;
  /** Extra footer rows, wrapped in their own `<tr>`s by the caller. */
  foot?: ReactNode;
  /**
   * Narrow layout: cards derived from the columns (default), a presenter of the
   * screen's own returning the whole item, or `false` to keep the table — dense
   * previews scroll instead.
   */
  card?: false | ((row: T) => ReactNode);
  /** Card list laid out as plain lines rather than boxes. */
  cards?: "cards" | "lines";
  /** Shown when there is nothing to render at all. */
  empty?: ReactNode;
  /** Ordering the table opens with; clicking a heading takes it from there. */
  defaultSort?: SortState;
  /**
   * Ordering held by the caller, for a table whose order outlives the screen. Given together
   * with `onSortChange`; without it the table keeps the order in its own state.
   */
  sort?: SortState | null;
  onSortChange?: (sort: SortState | null) => void;
}

function used<T>(columns: DataTableProps<T>["columns"], isWide: boolean): Column<T>[] {
  return (columns.filter(Boolean) as Column<T>[]).filter((c) => c.only !== "wide" || isWide);
}

/**
 * A heading is not a cell: a column with nothing to head has nothing for its class to style,
 * and `acts` on a `<th>` makes it `display: flex`, which the row then wraps in an anonymous
 * cell — a phantom column that widens the table and shifts every heading off its data.
 */
function headClasses<T>(column: Column<T>): string {
  const classes: string[] = [];
  if (column.align !== "left") classes.push("r", "num");
  if (column.className && column.header !== undefined) classes.push(column.className);
  return classes.join(" ");
}

function cellClasses<T>(column: Column<T>, row?: T): string {
  const classes: string[] = [];
  if (column.align !== "left") classes.push("r", "num");
  if (column.className) classes.push(column.className);
  const extra = row !== undefined ? column.cellClass?.(row) : undefined;
  if (extra) classes.push(extra);
  return classes.join(" ");
}

/** Card slot of a column, defaulting to first = title, last = value, rest = facts. */
function slotOf<T>(column: Column<T>, index: number, count: number): "title" | "value" | "fact" | "none" {
  if (column.card) return column.card;
  if (index === 0) return "title";
  if (index === count - 1) return "value";
  return "fact";
}

function Cards<T>({
  columns,
  sections,
  rowKey,
  rowProps,
  card,
  cards = "cards",
}: DataTableProps<T> & { sections: Section<T>[] }) {
  const cols = used(columns, false);
  const slot = (column: Column<T>, i: number) => slotOf(column, i, cols.length);

  const item = (row: T) =>
    card ? (
      card(row)
    ) : (
      <ListRow
        as="li"
        box={cards === "cards"}
        top
        title={cols.map((c, i) => (slot(c, i) === "title" ? c.cell(row) : null))}
        value={cols.map((c, i) => (slot(c, i) === "value" ? c.cell(row) : null))}
        foot={cols.map((c, i) =>
          slot(c, i) === "fact" ? (
            <span key={c.key}>
              {c.factLabel ?? c.header} {c.cell(row)}
            </span>
          ) : null,
        )}
        {...rowProps?.(row)}
      />
    );

  const list = (rows: T[]) => (
    <List as="ul" variant={cards === "cards" ? "cards" : "lines"}>
      {rows.map((row) => (
        <Fragment key={rowKey(row)}>{item(row)}</Fragment>
      ))}
    </List>
  );

  if (sections.length === 1 && !sections[0].cardHead) return list(sections[0].rows);
  return (
    <>
      {sections.map((section) => (
        <section className="section" key={section.key}>
          {section.cardHead}
          {list(section.rows)}
        </section>
      ))}
    </>
  );
}

export function DataTable<T>(props: DataTableProps<T>) {
  const { columns, rows, rowKey, rowProps, variant, fixed, sizing, className, foot, card, empty } = props;
  const isWide = useIsWide();
  const [scrollBox, overflowing] = useOverflow();
  const [own, setOwn] = useState<SortState | null>(props.defaultSort ?? null);
  // A caller that stores the order owns it; every other table keeps it for as long as it is open.
  const controlled = props.onSortChange !== undefined;
  const sort = controlled ? (props.sort ?? null) : own;
  const setSort = (next: SortState | null) => (controlled ? props.onSortChange!(next) : setOwn(next));
  const cols = used(columns, isWide);
  const sections = ordered(props.sections ?? [{ key: "all", rows: rows ?? [] }], columns, sort);
  const total = sections.reduce((n, section) => n + section.rows.length, 0);

  // A heading cycles its own column: first click the direction that reads the column best,
  // second the other one, third back to the order the screen handed over.
  const cycle = (column: Column<T>) => {
    const first = column.sortFirst ?? (column.align === "left" ? "asc" : "desc");
    if (sort?.key !== column.key) return setSort({ key: column.key, dir: first });
    if (sort.dir === first) return setSort({ key: column.key, dir: first === "asc" ? "desc" : "asc" });
    setSort(null);
  };

  if (total === 0 && empty !== undefined) return <>{empty}</>;
  if (!isWide && card !== false) return <Cards {...props} sections={sections} />;

  const classes = ["tbl"];
  if (variant) classes.push(`tbl--${variant}`);
  if (fixed) classes.push("tbl--fixed");
  if (sizing) classes.push(`tbl--${sizing}`);
  if (className) classes.push(className);

  const table = (
    <table className={classes.join(" ")}>
      <thead>
        <tr>
          {cols.map((column) => {
            const dir = sort?.key === column.key ? sort.dir : undefined;
            return (
              <th
                key={column.key}
                className={headClasses(column)}
                style={column.width ? { width: column.width } : undefined}
                aria-label={column.ariaLabel}
                data-tip={column.ariaLabel}
                aria-sort={column.sort ? (dir ? ARIA_SORT[dir] : "none") : undefined}
              >
                {column.sort ? (
                  <button
                    type="button"
                    className={`sortth${dir ? " sortth--on" : ""}`}
                    aria-label={column.ariaLabel}
                    onClick={() => cycle(column)}
                  >
                    {column.header}
                    <SortMark dir={dir} />
                  </button>
                ) : (
                  column.header
                )}
              </th>
            );
          })}
        </tr>
      </thead>
      <tbody>
        {sections.map((section) => (
          <SectionRows
            key={section.key}
            section={section}
            cols={cols}
            rowKey={rowKey}
            rowProps={rowProps}
            clamp={sizing === "content"}
          />
        ))}
      </tbody>
      {(foot || cols.some((column) => column.foot !== undefined)) && (
        <tfoot>
          {cols.some((column) => column.foot !== undefined) && (
            <tr>
              {cols.map((column) => (
                <td key={column.key} className={cellClasses(column)}>
                  {column.foot}
                </td>
              ))}
            </tr>
          )}
          {foot}
        </tfoot>
      )}
    </table>
  );

  // `content` sizing states that the caller owns the scrolling (`Scrolly x`); everything
  // else is wrapped here, so no screen has to remember that a table can outgrow its panel.
  if (sizing === "content") return table;
  return (
    <div ref={scrollBox} className={`tbl-wrap${overflowing ? " tbl-wrap--over" : ""}`}>
      {table}
    </div>
  );
}

const ARIA_SORT: Record<SortDir, "ascending" | "descending"> = {
  asc: "ascending",
  desc: "descending",
};

function SortMark({ dir }: { dir?: SortDir }) {
  const Icon = dir === "asc" ? CaretUpIcon : dir === "desc" ? CaretDownIcon : CaretUpDownIcon;
  return <Icon className="sortth__mark" weight="bold" aria-hidden />;
}

/** Rows are ordered inside each section: a sort re-reads a group, it never dissolves it. */
function ordered<T>(
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

/**
 * An absent value sorts last whichever way the column points — "no price" is not "the
 * lowest price" — so it is settled before the direction is applied at all.
 */
function compare(a: SortValue, b: SortValue, dir: number): number {
  const missingA = a === null || a === undefined || a === "" || (typeof a === "number" && Number.isNaN(a));
  const missingB = b === null || b === undefined || b === "" || (typeof b === "number" && Number.isNaN(b));
  if (missingA || missingB) return missingA && missingB ? 0 : missingA ? 1 : -1;
  if (typeof a === "number" && typeof b === "number") return dir * (a - b);
  if (typeof a === "boolean" && typeof b === "boolean") return dir * (Number(a) - Number(b));
  // `numeric` so a column of strings that happen to be numbers ("10") still reads as numbers.
  return dir * String(a).localeCompare(String(b), currentLocale(), { numeric: true });
}

/**
 * Sideways scrolling only once the columns really do not fit. Measured rather than assumed
 * because a scroll container is also a sticky container: while the table fits, the wrapper
 * stays a plain box and the sticky header keeps sticking to the page.
 */
function useOverflow() {
  const boxRef = useRef<HTMLDivElement>(null);
  const [over, setOver] = useState(false);

  useEffect(() => {
    const box = boxRef.current;
    if (!box) return;
    // Fires once on observe, so the first measurement needs no setState of its own.
    const observer = new ResizeObserver(() => setOver(box.scrollWidth > box.clientWidth + 1));
    observer.observe(box);
    if (box.firstElementChild) observer.observe(box.firstElementChild);
    return () => observer.disconnect();
  }, []);

  return [boxRef, over] as const;
}

function SectionRows<T>({
  section,
  cols,
  rowKey,
  rowProps,
  clamp,
}: {
  section: Section<T>;
  cols: Column<T>[];
  rowKey: (row: T) => string;
  rowProps?: (row: T) => HTMLAttributes<HTMLTableRowElement>;
  /** Wraps every cell so a capped column can cut its text — `sizing="content"` only. */
  clamp?: boolean;
}) {
  return (
    <>
      {section.heads?.map((head) => (
        <tr className={`grp${head.className ? ` ${head.className}` : ""}`} key={head.key}>
          {head.cells}
        </tr>
      ))}
      {section.rows.map((row) => (
        <tr key={rowKey(row)} {...rowProps?.(row)}>
          {cols.map((column) => (
            <td key={column.key} className={cellClasses(column, row)}>
              {clamp && column.clamp !== false ? (
                <span className="cell">{column.cell(row)}</span>
              ) : (
                column.cell(row)
              )}
            </td>
          ))}
        </tr>
      ))}
    </>
  );
}
