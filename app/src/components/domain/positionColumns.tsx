import type { ReactNode } from "react";
import type { I18n } from "@lingui/core";
import type { MessageDescriptor } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";
import { useAttributeDefs } from "../../lib/queries";
import { Money, Num, Percent, Quantity, Rate, type SortValue } from "../ui";
import { formatDay, toneClass, toNumber } from "../../lib/format";
import { DIVIDEND_FREQUENCY_LABELS } from "../../lib/kinds";
import type {
  CostBasisFigures,
  CostBasisMethod,
  PositionCostRow,
  PositionReturnRow,
  PositionRow,
  SecurityAttributeDef,
  SecurityRow,
} from "../../lib/types";
import { QuoteSpark } from "./QuoteSpark";
import { SecurityLink } from "./SecurityCardProvider";

/**
 * The groups the column picker lays its list out in, in that order. A group is what the
 * figure is read off — a quote, the holding, the instrument's record — because two columns
 * with almost the same name (a reported yield and a received one) are told apart by that
 * and by nothing else.
 */
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

export const COLUMNS: Column[] = [
  {
    id: "spark",
    group: "quotes",
    width: "96px",
    label: (i18n) => i18n._(msg`90 days`),
    tip: (i18n) => i18n._(msg`The last 90 closes, whatever period the screen is set to.`),
    cell: (row) => <QuoteSpark securityId={row.security_id} symbol={row.symbol} />,
    align: "left",
  },
  {
    id: "quantity",
    group: "value",
    sort: (row) => toNumber(row.quantity),
    width: "100px",
    label: (i18n) => i18n._(msg`Qty`),
    cell: (row) => <Quantity value={row.quantity} />,
  },
  {
    // Keep quote currency adjacent to the price without repeating it in every value.
    id: "price",
    group: "quotes",
    sort: (row) => toNumber(row.price),
    width: "104px",
    label: (i18n) => i18n._(msg`Price`),
    cell: (row) => (
      <>
        <Money value={row.price} />
        <span className="cur">{row.currency}</span>
      </>
    ),
  },
  {
    id: "fx",
    group: "quotes",
    sort: (row) => toNumber(row.fx_rate),
    width: "80px",
    label: (i18n) => i18n._(msg`FX rate`),
    tip: (i18n) => i18n._(msg`Today's rate from the quote currency into the base one.`),
    cell: (row) => <Money value={row.fx_rate} digits={4} />,
  },
  {
    id: "value",
    group: "value",
    sort: (row) => toNumber(row.market_value_base),
    width: "104px",
    label: (i18n, currency) => i18n._(msg`Value, ${currency}`),
    cell: (row) => <Money value={row.market_value_base} />,
  },
  {
    id: "weight",
    group: "value",
    sort: (row) => toNumber(row.weight),
    width: "78px",
    label: (i18n) => i18n._(msg`Weight`),
    tip: (i18n) => i18n._(msg`This position's share of everything the current scope holds.`),
    cell: (row) => <Percent value={row.weight} digits={1} />,
  },
  {
    id: "day",
    group: "quotes",
    sort: (row) => toNumber(row.day_change),
    width: "86px",
    label: (i18n) => i18n._(msg`Today`),
    tip: (i18n) => i18n._(msg`Move from the previous close to the last one.`),
    cell: (row) => (row.day_change ? <Percent value={row.day_change} signed tone={false} /> : "—"),
    tone: (row) => (row.day_change ? toneClass(row.day_change) : ""),
  },
  {
    // The two halves of the unrealised result: what the instrument did, and what the rate did to it.
    id: "instgain",
    group: "result",
    sort: (row) => toNumber(row.instrument_gain_base),
    width: "104px",
    label: (i18n) => i18n._(msg`Instr. gain`),
    tip: (i18n) => i18n._(msg`The instrument's own share of the unrealised result.`),
    cell: (row) => <Money value={row.instrument_gain_base} signed tone={false} />,
    tone: (row) => toneClass(row.instrument_gain_base),
  },
  {
    id: "curgain",
    group: "result",
    sort: (row) => toNumber(row.currency_gain_base),
    width: "104px",
    label: (i18n) => i18n._(msg`FX gain`),
    tip: (i18n) => i18n._(msg`What the exchange rate did to the unrealised result.`),
    cell: (row) => <Money value={row.currency_gain_base} signed tone={false} />,
    tone: (row) => toneClass(row.currency_gain_base),
  },
  {
    id: "dividends",
    group: "income",
    sort: (row) => toNumber(row.dividends_base),
    width: "104px",
    label: (i18n) => i18n._(msg`Dividends received`),
    tip: (i18n) => i18n._(msg`What this position has actually been paid, in the base currency.`),
    cell: (row) => <Money value={row.dividends_base} />,
  },
  {
    // Last year's payments over today's value: the rate the position pays now.
    id: "divyield",
    group: "income",
    sort: (row) => toNumber(row.dividend_yield),
    width: "92px",
    label: (i18n) => i18n._(msg`Yield received`),
    tip: (i18n) => i18n._(msg`Last year's payments over today's value.`),
    cell: (row) => (row.dividend_yield ? <Percent value={row.dividend_yield} digits={2} /> : dash()),
  },
  {
    id: "yieldcost",
    group: "income",
    sort: (row) => toNumber(row.yield_on_cost),
    width: "104px",
    label: (i18n) => i18n._(msg`Yield on cost`),
    tip: (i18n) => i18n._(msg`Everything this holding has ever paid, over what the shares cost.`),
    cell: (row) => (row.yield_on_cost ? <Percent value={row.yield_on_cost} digits={2} /> : dash()),
  },
  {
    // Read off the whole payment history, so it names the instrument and not the window.
    id: "divfreq",
    group: "income",
    sort: (row, ctx) => {
      const label = DIVIDEND_FREQUENCY_LABELS[row.dividend_frequency];
      return label ? ctx.i18n._(label) : null;
    },
    width: "120px",
    label: (i18n) => i18n._(msg`Pays`),
    tip: (i18n) => i18n._(msg`Schedule read off the whole payment history, not the chosen period.`),
    align: "left",
    cell: (row, ctx) => {
      const label = DIVIDEND_FREQUENCY_LABELS[row.dividend_frequency];
      return label ? ctx.i18n._(label) : dash();
    },
  },
  {
    id: "divlast",
    group: "income",
    sort: (row) => row.dividend_last,
    width: "116px",
    label: (i18n) => i18n._(msg`Last received`),
    tip: (i18n) => i18n._(msg`Date this position was last paid.`),
    cell: (row) => (row.dividend_last ? formatDay(row.dividend_last) : dash()),
  },
  {
    // The high of the quotes we store, which is not necessarily the instrument's own.
    id: "ath",
    group: "quotes",
    sort: (row) => toNumber(row.ath_distance),
    width: "96px",
    label: (i18n) => i18n._(msg`From high`),
    tip: (i18n) => i18n._(msg`Distance to the highest close stored here, not the instrument's own record.`),
    cell: (row) => (row.ath_distance ? <Percent value={row.ath_distance} digits={1} tone={false} /> : dash()),
    tone: (row) => (row.ath_distance ? toneClass(row.ath_distance) : ""),
  },
  {
    id: "athprice",
    group: "quotes",
    sort: (row) => toNumber(row.ath_price),
    width: "104px",
    label: (i18n) => i18n._(msg`All-time high`),
    tip: (i18n) => i18n._(msg`Highest close stored here, in the quote currency.`),
    cell: (row) =>
      row.ath_price ? (
        <>
          <Money value={row.ath_price} />
          <span className="cur">{row.currency}</span>
        </>
      ) : (
        dash()
      ),
  },
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
  ...costColumns("fifo", msg`FIFO`),
  ...costColumns("average", msg`moving avg`),
  ...recordColumns<PositionRow, CellCtx>(),
  {
    id: "accounts",
    group: "record",
    sort: (row) => row.accounts.map((a) => a.account_name).join(", "),
    width: "160px",
    label: (i18n) => i18n._(msg`Accounts`),
    // Lots do not identify their depot; the position's accounts do.
    cell: (row) => row.accounts.map((a) => a.account_name).join(", ") || "—",
  },
];

/** The two methods, by the suffix their column ids carry. */
const METHOD_OF: Record<CostBasisMethod, "fifo" | "average"> = {
  FIFO: "fifo",
  AVERAGE_COST: "average",
};

/**
 * Purchase figures are offered under a named method only: a column labelled neither one
 * would change meaning with a setting, and could then not be compared with the one beside it.
 * The method the portfolio is kept under is already in the position row, so only the *other*
 * one costs the second holdings pass — `needsCostQuery` is what decides that.
 */
function costColumns(method: "fifo" | "average", name: MessageDescriptor): Column[] {
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

/**
 * A stored choice from before purchase figures named their method: it meant "whatever the
 * portfolio is kept under", so it resolves to that method's column rather than being dropped.
 */
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

/**
 * Whether the shown columns need the cost-basis comparison: the portfolio's own method is
 * already in the position row, so only the other method — or a unit price, which no row
 * carries — is worth a second holdings pass.
 */
export function needsCostQuery(ids: string[], method: CostBasisMethod | undefined): boolean {
  const own = METHOD_OF[method ?? "FIFO"];
  return ids.some((id) => {
    if (id.startsWith("costunit-")) return true;
    const other = own === "fifo" ? "average" : "fifo";
    return id.endsWith(`-${other}`);
  });
}

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

/** A period figure is absent for two different reasons and both read as one dash: the query
 * has not answered yet, or the return had no root. A plain helper, not a component — it holds
 * no hooks, and a component in this file would cost fast refresh. */
export function periodCell(value: string | null | undefined): ReactNode {
  if (!value) return dash();
  return <Percent value={value} signed tone={false} />;
}

/** A figure the cost-basis query has not answered for yet reads as an absent one. */
function money(value: string | undefined, signed = false): ReactNode {
  if (value === undefined) return dash();
  return <Money value={value} signed={signed} tone={false} />;
}

/** An absent figure, dimmed so a column of them does not read as data. */
export function dash(): ReactNode {
  return <span className="dim">—</span>;
}
