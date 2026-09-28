import { msg, plural } from "@lingui/core/macro";
import { Money, Num, Rate } from "../../ui";
import { toneClass, toNumber } from "../../../lib/format";
import type { Column } from "./types";
import { dash, periodCell } from "./cells";

/** Figures over the chosen period: they read the position's return row, not the position. */
export const PERIOD_COLUMNS: Column[] = [
  {
    id: "twr",
    group: "result",
    sort: (_row, ctx) => toNumber(ctx.period?.twr),
    width: "92px",
    label: (i18n) => i18n._(msg`TWR`),
    tip: (i18n) => i18n._(msg`Return of the instrument itself, unaffected by what was paid in or out.`),
    cell: (_row, ctx) => periodCell(ctx.period?.twr),
    tone: (_row, ctx) => (ctx.period?.twr ? toneClass(ctx.period.twr) : ""),
  },
  {
    id: "twrann",
    group: "result",
    sort: (_row, ctx) => toNumber(ctx.period?.twr_annualized),
    width: "100px",
    label: (i18n) => i18n._(msg`TWR a year`),
    tip: (i18n) => i18n._(msg`The period's time-weighted return restated as a yearly rate.`),
    cell: (_row, ctx) => periodCell(ctx.period?.twr_annualized),
    tone: (_row, ctx) => (ctx.period?.twr_annualized ? toneClass(ctx.period.twr_annualized) : ""),
  },
  {
    id: "irr",
    group: "result",
    sort: (_row, ctx) => toNumber(ctx.period?.xirr),
    width: "92px",
    label: (i18n) => i18n._(msg`IRR`),
    tip: (i18n) => i18n._(msg`Yearly rate that makes this position's payments add up to what it is worth.`),
    cell: (_row, ctx) => periodCell(ctx.period?.xirr),
    tone: (_row, ctx) => (ctx.period?.xirr ? toneClass(ctx.period.xirr) : ""),
  },
  {
    // Not TWR: this one divides by the money the position itself had at work.
    id: "absperf",
    group: "result",
    sort: (_row, ctx) => toNumber(ctx.period?.absolute_performance),
    width: "104px",
    label: (i18n) => i18n._(msg`Perf. %`),
    tip: (i18n) => i18n._(msg`The period's result over the money this position had at work.`),
    cell: (_row, ctx) => periodCell(ctx.period?.absolute_performance),
    tone: (_row, ctx) => (ctx.period?.absolute_performance ? toneClass(ctx.period.absolute_performance) : ""),
  },
  {
    id: "result",
    group: "result",
    sort: (_row, ctx) => toNumber(ctx.period?.pnl_base),
    width: "104px",
    label: (i18n) => i18n._(msg`Result`),
    tip: (i18n) => i18n._(msg`What the position made over the chosen period, in the base currency.`),
    cell: (_row, ctx) => (ctx.period ? <Money value={ctx.period.pnl_base} signed tone={false} /> : "—"),
    tone: (_row, ctx) => (ctx.period ? toneClass(ctx.period.pnl_base) : ""),
  },
  {
    // Trade commissions included, which is why this differs from the charges report.
    id: "fees",
    group: "costs",
    sort: (_row, ctx) => toNumber(ctx.period?.fees_base),
    width: "96px",
    label: (i18n) => i18n._(msg`Fees`),
    tip: (i18n) => i18n._(msg`Trade commissions included, so it exceeds what the charges report lists.`),
    cell: (_row, ctx) => (ctx.period ? <Money value={ctx.period.fees_base} /> : "—"),
  },
  {
    id: "taxes",
    group: "costs",
    sort: (_row, ctx) => toNumber(ctx.period?.taxes_base),
    width: "96px",
    label: (i18n) => i18n._(msg`Taxes`),
    cell: (_row, ctx) => (ctx.period ? <Money value={ctx.period.taxes_base} /> : "—"),
  },
  {
    // The position's own daily returns over the period, the ones its TWR chains.
    id: "vol",
    group: "risk",
    sort: (_row, ctx) => ctx.period?.risk?.volatility,
    width: "96px",
    label: (i18n) => i18n._(msg`Volatility`),
    tip: (i18n) => i18n._(msg`How widely this position's daily returns swing, stated for a year.`),
    cell: (_row, ctx) => (ctx.period?.risk ? <Rate value={ctx.period.risk.volatility} /> : dash()),
  },
  {
    id: "semidev",
    group: "risk",
    sort: (_row, ctx) => ctx.period?.risk?.semi_deviation,
    width: "104px",
    label: (i18n) => i18n._(msg`Semi-deviation`),
    tip: (i18n) => i18n._(msg`How much of the same swing the losing days account for.`),
    cell: (_row, ctx) => (ctx.period?.risk ? <Rate value={ctx.period.risk.semi_deviation} /> : dash()),
  },
  {
    id: "maxdd",
    group: "risk",
    sort: (_row, ctx) => ctx.period?.risk?.max_drawdown,
    width: "104px",
    label: (i18n) => i18n._(msg`Max drawdown`),
    tip: (i18n) => i18n._(msg`Deepest fall from a peak inside the period.`),
    cell: (_row, ctx) =>
      ctx.period?.risk ? <Rate value={ctx.period.risk.max_drawdown} signed tone={false} /> : dash(),
    tone: (_row, ctx) => (ctx.period?.risk?.max_drawdown ? "neg" : ""),
  },
  {
    id: "mdddays",
    group: "risk",
    sort: (_row, ctx) => ctx.period?.risk?.max_drawdown_days ?? undefined,
    width: "104px",
    label: (i18n) => i18n._(msg`Drawdown length`),
    tip: (i18n) => i18n._(msg`Days the deepest fall lasted before the peak was regained.`),
    cell: (_row, ctx) => {
      const days = ctx.period?.risk?.max_drawdown_days;
      return days == null ? dash() : <Num>{plural(days, { one: "# day", other: "# days" })}</Num>;
    },
  },
];
