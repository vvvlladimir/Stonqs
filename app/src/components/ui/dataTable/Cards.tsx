import { Fragment } from "react";
import { List, ListRow } from "../List";
import type { Column, Section, DataTableProps } from "./types";
import { used, slotOf } from "./model";

export function Cards<T>({
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
