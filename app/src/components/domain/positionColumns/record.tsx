import { msg } from "@lingui/core/macro";
import { toNumber } from "../../../lib/format";
import type { SecurityAttributeDef } from "../../../lib/types";
import { SecurityLink } from "../SecurityCardProvider";
import type { ColumnOf, RecordCtx } from "./types";
import { dash } from "./cells";

/** The instrument's own codes and note: the same three columns in every table that has a row
 * per instrument, read to be looked at rather than edited — that is the directory's job. */
export function recordColumns<Row extends { security_id: string }, Ctx extends RecordCtx>(): Array<
  ColumnOf<Row, Ctx>
> {
  return [
    {
      id: "isin",
      group: "record",
      sort: (_row, ctx) => ctx.security?.isin,
      width: "136px",
      label: (i18n) => i18n._(msg`ISIN`),
      align: "left",
      cell: (row, ctx) =>
        ctx.security?.isin ? <SecurityLink id={row.security_id}>{ctx.security.isin}</SecurityLink> : dash(),
    },
    {
      id: "wkn",
      group: "record",
      sort: (_row, ctx) => ctx.security?.wkn,
      width: "96px",
      label: (i18n) => i18n._(msg`WKN`),
      align: "left",
      cell: (_row, ctx) => ctx.security?.wkn ?? dash(),
    },
    {
      id: "note",
      group: "record",
      sort: (_row, ctx) => ctx.security?.note,
      width: "220px",
      label: (i18n) => i18n._(msg`Note`),
      align: "left",
      cell: (_row, ctx) => ctx.security?.note ?? dash(),
    },
  ];
}

/** One column per attribute the user defined. The label is user data and is not translated;
 * the id keeps the attribute's own id, so a renamed attribute keeps its place in the table. */
export function attributeColumns<Row, Ctx extends RecordCtx>(
  defs: SecurityAttributeDef[],
): Array<ColumnOf<Row, Ctx>> {
  return defs.map((def) => ({
    id: `attr:${def.id}`,
    group: "attrs" as const,
    width: "140px",
    label: () => def.name,
    align: "left" as const,
    cell: (_row: Row, ctx: Ctx) => {
      const value = ctx.security?.attributes[def.id];
      return value ? [value, def.unit].filter(Boolean).join(" ") : dash();
    },
    // A number is stored as text like every other attribute; ordering it as text would put 10 before 9.
    sort: (_row: Row, ctx: Ctx) => {
      const value = ctx.security?.attributes[def.id];
      return def.kind === "NUMBER" ? toNumber(value) : value;
    },
  }));
}
