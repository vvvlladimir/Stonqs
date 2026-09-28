import { CaretDownIcon, CaretUpDownIcon, CaretUpIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState, type HTMLAttributes } from "react";
import { useIsWide } from "../../lib/useLayout";
import { FIELDS, focusables } from "./focus";
import type { Column, DataTableProps, Section, SortDir, SortState } from "./dataTable/types";
import { used, headClasses, cellClasses, ordered } from "./dataTable/model";
import { Cards } from "./dataTable/Cards";

export type { SortDir, SortValue, SortState, Column, Section, DataTableProps } from "./dataTable/types";

/** The only `<table>`: columns in, markup, widths, ordering and the narrow card layout out. */
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
      <tbody onKeyDown={moveInRows}>
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

/** ↑/↓ move to the same column of the next row; inputs keep their own arrows. */
function moveInRows(e: React.KeyboardEvent<HTMLElement>) {
  if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
  if (e.altKey || e.metaKey || e.ctrlKey || e.shiftKey || e.defaultPrevented) return;
  const from = e.target as HTMLElement;
  if (from.matches(FIELDS)) return;
  const row = from.closest("tr");
  const cell = from.closest("td");
  if (!row || !cell) return;
  const column = [...row.children].indexOf(cell);
  const down = e.key === "ArrowDown";
  for (let next = down ? row.nextElementSibling : row.previousElementSibling; next;) {
    const target =
      focusables((next.children[column] as HTMLElement) ?? next)[0] ?? focusables(next as HTMLElement)[0];
    if (target) {
      e.preventDefault();
      target.focus();
      target.scrollIntoView({ block: "nearest" });
      return;
    }
    next = down ? next.nextElementSibling : next.previousElementSibling;
  }
}

const ARIA_SORT: Record<SortDir, "ascending" | "descending"> = {
  asc: "ascending",
  desc: "descending",
};

function SortMark({ dir }: { dir?: SortDir }) {
  const Icon = dir === "asc" ? CaretUpIcon : dir === "desc" ? CaretDownIcon : CaretUpDownIcon;
  return <Icon className="sortth__mark" weight="bold" aria-hidden />;
}

/** Scrolls sideways only when the columns really overflow, so the sticky header keeps working. */
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
