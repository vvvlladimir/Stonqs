import type { I18n, MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { Money } from "../../ui";
import { toneClass, toNumber } from "../../../lib/format";
import type { CostBasisFigures, CostBasisMethod } from "../../../lib/types";
import type { CellCtx, Column } from "./types";
import { money, dash } from "./cells";

/** The two methods, by the suffix their column ids carry. */
const METHOD_OF: Record<CostBasisMethod, "fifo" | "average"> = {
  FIFO: "fifo",
  AVERAGE_COST: "average",
};

/** Purchase figures only under a named method, so a column never changes meaning with a setting. */
export function costColumns(method: "fifo" | "average", name: MessageDescriptor): Column[] {
  const own = (ctx: CellCtx) => ctx.own !== undefined && METHOD_OF[ctx.own] === method;
  const of = (ctx: CellCtx): CostBasisFigures | undefined => ctx.cost?.[method];
  const label = (i18n: I18n, text: string) => `${text} (${i18n._(name)})`;
  /** The row already holds this figure when it is the portfolio's own method. */
  const figure = (ctx: CellCtx, key: keyof CostBasisFigures, from: string) =>
    own(ctx) && !ctx.cost ? from : of(ctx)?.[key];
  return [
    {
      id: `cost-${method}`,
      group: "value",
      sort: (row, ctx) => toNumber(figure(ctx, "cost_basis_base", row.cost_basis_base)),
      width: "128px",
      label: (i18n, currency) => label(i18n, i18n._(msg`Purchase value, ${currency}`)),
      tip: (i18n) => i18n._(msg`What the shares held today cost, counted under this method.`),
      cell: (row, ctx) => money(figure(ctx, "cost_basis_base", row.cost_basis_base)),
    },
    {
      id: `costunit-${method}`,
      group: "value",
      sort: (_row, ctx) => toNumber(of(ctx)?.cost_per_unit),
      width: "128px",
      label: (i18n) => label(i18n, i18n._(msg`Purchase price`)),
      tip: (i18n) => i18n._(msg`Average cost of one share, in the currency it was paid for.`),
      // In the currency the shares were paid for, which is what a broker statement shows.
      cell: (row, ctx) => {
        const figures = of(ctx);
        if (!figures) return dash();
        return (
          <>
            <Money value={figures.cost_per_unit} />
            <span className="cur">{ctx.cost?.cost_currency ?? row.cost_currency}</span>
          </>
        );
      },
    },
    {
      id: `unreal-${method}`,
      group: "result",
      sort: (row, ctx) => toNumber(figure(ctx, "unrealized_pnl_base", row.unrealized_pnl_base)),
      width: "128px",
      label: (i18n) => label(i18n, i18n._(msg`Unrealised`)),
      tip: (i18n) => i18n._(msg`Value today less what the shares cost under this method.`),
      cell: (row, ctx) => money(figure(ctx, "unrealized_pnl_base", row.unrealized_pnl_base), true),
      tone: (row, ctx) => toneClass(figure(ctx, "unrealized_pnl_base", row.unrealized_pnl_base) ?? "0"),
    },
    {
      id: `real-${method}`,
      group: "result",
      sort: (row, ctx) => toNumber(figure(ctx, "realized_pnl_base", row.realized_pnl_base)),
      width: "128px",
      label: (i18n) => label(i18n, i18n._(msg`Realised`)),
      tip: (i18n) => i18n._(msg`Result of what has already been sold, counted under this method.`),
      cell: (row, ctx) => money(figure(ctx, "realized_pnl_base", row.realized_pnl_base), true),
      tone: (row, ctx) => toneClass(figure(ctx, "realized_pnl_base", row.realized_pnl_base) ?? "0"),
    },
  ];
}

/** Pre-naming choices resolve to the portfolio's own method. */
const LEGACY: Record<string, string> = { cost: "cost", unrealized: "unreal", realized: "real" };

export function resolveColumnIds(ids: string[], method: CostBasisMethod | undefined): string[] {
  if (!ids.some((id) => id in LEGACY)) return ids;
  const suffix = METHOD_OF[method ?? "FIFO"];
  const out: string[] = [];
  for (const id of ids) {
    const resolved = id in LEGACY ? `${LEGACY[id]}-${suffix}` : id;
    if (!out.includes(resolved)) out.push(resolved);
  }
  return out;
}

/** Only the other method, or a unit price, needs the second holdings pass. */
export function needsCostQuery(ids: string[], method: CostBasisMethod | undefined): boolean {
  const own = METHOD_OF[method ?? "FIFO"];
  return ids.some((id) => {
    if (id.startsWith("costunit-")) return true;
    const other = own === "fifo" ? "average" : "fifo";
    return id.endsWith(`-${other}`);
  });
}
