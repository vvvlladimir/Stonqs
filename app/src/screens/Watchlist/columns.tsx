import type { I18n } from "@lingui/core";
import type { ReactNode } from "react";
import { msg } from "@lingui/core/macro";
import { Bar, Money, Percent } from "../../components/ui";
import {
  COLUMNS,
  attributeColumns,
  dash,
  recordColumns,
  type Column,
  type ColumnOf,
} from "../../components/domain/positionColumns";
import { QuoteSpark } from "../../components/domain/QuoteSpark";
import { useAttributeDefs } from "../../lib/queries";
import { formatDay, toNumber, toneClass } from "../../lib/format";
import { toPlotNumber } from "../../lib/plot";
import type {
  CostBasisMethod,
  DateString,
  PositionCostRow,
  PositionReturnRow,
  PositionRow,
  SecurityRow,
  WatchRow,
} from "../../lib/types";

/** What a cell may read besides the instrument's own figures. */
export interface WatchCtx {
  /** Base currency: a held position's figures are in it. */
  currency: string;
  i18n: I18n;
  /** Where the chosen period begins; a later `period_start` means a younger instrument. */
  from?: DateString;
  /** The position held in the instrument under the picker's scope; absent when none is. */
  position?: PositionRow;
  period?: PositionReturnRow;
  security?: SecurityRow;
  /** Both cost-basis methods for that position, and the one the portfolio is kept under. */
  cost?: PositionCostRow;
  own?: CostBasisMethod;
}

export type WatchColumn = ColumnOf<WatchRow, WatchCtx>;

/** A price in the instrument's own currency, which the column header cannot name. */
function price(value: string | null, row: WatchRow, title?: string): ReactNode {
  if (!value) return dash();
  return (
    <>
      <Money value={value} title={title} />
      <span className="cur">{row.currency}</span>
    </>
  );
}

function change(value: string | null, title?: string): ReactNode {
  return value ? <Percent value={value} signed tone={false} title={title} /> : dash();
}

const tone = (value: string | null) => (value ? toneClass(value) : "");

/** Figures read off the instrument's own quotes: they exist whether it is held or not. */
const QUOTE: WatchColumn[] = [
  {
    id: "spark",
    group: "quotes",
    width: "96px",
    label: (i18n) => i18n._(msg`90 days`),
    tip: (i18n) => i18n._(msg`The last 90 closes, whatever period the screen is set to.`),
    align: "left",
    cell: (row) => <QuoteSpark securityId={row.security_id} symbol={row.symbol} />,
  },
  {
    id: "price",
    group: "quotes",
    sort: (row) => toNumber(row.price),
    width: "112px",
    label: (i18n) => i18n._(msg`Price`),
    cell: (row) => price(row.price, row, row.price_date ? formatDay(row.price_date) : undefined),
  },
  {
    id: "pricedate",
    group: "quotes",
    sort: (row) => row.price_date,
    width: "104px",
    label: (i18n) => i18n._(msg`Quote date`),
    tip: (i18n) => i18n._(msg`Day the last stored close belongs to.`),
    cell: (row) => (row.price_date ? formatDay(row.price_date) : dash()),
  },
  {
    id: "day",
    group: "quotes",
    sort: (row) => toNumber(row.day_change),
    width: "86px",
    label: (i18n) => i18n._(msg`Today`),
    tip: (i18n) => i18n._(msg`Move from the previous close to the last one.`),
    cell: (row) => change(row.day_change),
    tone: (row) => tone(row.day_change),
  },
  {
    // An instrument younger than the period is measured from its first close, and says so.
    id: "period",
    group: "quotes",
    sort: (row) => toNumber(row.period_return),
    width: "96px",
    label: (i18n) => i18n._(msg`Period`),
    tip: (i18n) => i18n._(msg`Price move over the chosen period, whether the instrument is held or not.`),
    cell: (row, ctx) => {
      const start = row.period_start;
      const late = start && ctx.from && start > ctx.from ? formatDay(start) : null;
      return change(row.period_return, late ? ctx.i18n._(msg`Since ${late}: no earlier quote`) : undefined);
    },
    tone: (row) => tone(row.period_return),
  },
  {
    id: "low",
    group: "quotes",
    sort: (row) => toNumber(row.low),
    width: "112px",
    label: (i18n) => i18n._(msg`Period low`),
    cell: (row) => price(row.low, row),
  },
  {
    id: "high",
    group: "quotes",
    sort: (row) => toNumber(row.high),
    width: "112px",
    label: (i18n) => i18n._(msg`Period high`),
    cell: (row) => price(row.high, row),
  },
  {
    id: "range",
    group: "quotes",
    sort: (row) => toNumber(row.range_position),
    width: "112px",
    label: (i18n) => i18n._(msg`In range`),
    tip: (i18n) => i18n._(msg`Where the price stands between the period's low and high.`),
    align: "left",
    // A position on a track, not an amount: the tick is drawn, never added up.
    cell: (row, ctx) =>
      row.range_position ? (
        <Bar
          size="sm"
          tick={`${toPlotNumber(row.range_position) * 100}%`}
          label={ctx.i18n._(msg`Where the price stands between the period's low and high`)}
        />
      ) : (
        dash()
      ),
  },
  {
    id: "level",
    group: "levels",
    sort: (row) => toNumber(row.nearest_level?.level),
    width: "112px",
    label: (i18n) => i18n._(msg`Nearest level`),
    tip: (i18n) => i18n._(msg`The alert level closest to the price right now.`),
    cell: (row) => {
      const level = row.nearest_level;
      if (!level) return dash();
      return (
        <>
          <Money value={level.level} />
          <span className="cur">{level.currency}</span>
        </>
      );
    },
  },
  {
    id: "leveldist",
    group: "levels",
    sort: (row) => toNumber(row.nearest_level?.distance),
    width: "96px",
    label: (i18n) => i18n._(msg`To level`),
    tip: (i18n) => i18n._(msg`How far the price is from that level.`),
    cell: (row) => change(row.nearest_level?.distance ?? null),
  },
  {
    id: "ath",
    group: "quotes",
    sort: (row) => toNumber(row.ath_distance),
    width: "96px",
    label: (i18n) => i18n._(msg`From high`),
    tip: (i18n) => i18n._(msg`Distance to the highest close stored here, not the instrument's own record.`),
    cell: (row) => (row.ath_distance ? <Percent value={row.ath_distance} digits={1} tone={false} /> : dash()),
    tone: (row) => tone(row.ath_distance),
  },
  {
    id: "athprice",
    group: "quotes",
    sort: (row) => toNumber(row.ath_price),
    width: "112px",
    label: (i18n) => i18n._(msg`All-time high`),
    tip: (i18n) => i18n._(msg`Highest close stored here, in the quote currency.`),
    cell: (row) => price(row.ath_price, row, row.ath_date ? formatDay(row.ath_date) : undefined),
  },
  {
    // What the provider reported, per share — not what the portfolio received.
    id: "repdiv",
    group: "income",
    sort: (row) => toNumber(row.dividend_year),
    width: "112px",
    label: (i18n) => i18n._(msg`Dividend a year`),
    tip: (i18n) => i18n._(msg`Per share, as the data source reported it, not what the portfolio was paid.`),
    cell: (row) => price(row.dividend_year, row),
  },
  {
    id: "repyield",
    group: "income",
    sort: (row) => toNumber(row.dividend_yield),
    width: "96px",
    label: (i18n) => i18n._(msg`Dividend yield`),
    tip: (i18n) => i18n._(msg`Reported payments of the last year over today's price.`),
    cell: (row) => (row.dividend_yield ? <Percent value={row.dividend_yield} digits={2} /> : dash()),
  },
  {
    id: "exdate",
    group: "income",
    sort: (row) => row.dividend_last,
    width: "116px",
    label: (i18n) => i18n._(msg`Last ex-date`),
    tip: (i18n) => i18n._(msg`Last ex-dividend date the data source reported.`),
    cell: (row) => (row.dividend_last ? formatDay(row.dividend_last) : dash()),
  },
];

/** Positions columns that describe a holding; a watched instrument nobody holds shows a dash. */
const HOLDING = new Set([
  "quantity",
  "fx",
  "value",
  "weight",
  "instgain",
  "curgain",
  "dividends",
  "divyield",
  "yieldcost",
  "divfreq",
  "divlast",
  "cost-fifo",
  "cost-average",
  "costunit-fifo",
  "costunit-average",
  "unreal-fifo",
  "unreal-average",
  "real-fifo",
  "real-average",
  "twr",
  "twrann",
  "irr",
  "absperf",
  "result",
  "fees",
  "taxes",
  "vol",
  "semidev",
  "maxdd",
  "mdddays",
  "accounts",
]);

/**
 * A positions column read through the watchlist's row: every one of them describes the
 * holding, so they are filed under one group however the positions table files them.
 */
function held(column: Column): WatchColumn {
  return {
    id: column.id,
    group: "holding",
    width: column.width,
    align: column.align,
    label: column.label,
    tip: column.tip,
    cell: (_row, ctx) => (ctx.position ? column.cell(ctx.position, ctx) : dash()),
    sort:
      column.sort &&
      ((_row: WatchRow, ctx: WatchCtx) => (ctx.position ? column.sort!(ctx.position, ctx) : null)),
    tone: (_row, ctx) => (ctx.position && column.tone ? column.tone(ctx.position, ctx) : ""),
  };
}

/** Every column the watchlist can offer: quote figures, the holding's, then the record's. */
export function useWatchColumns(): WatchColumn[] {
  const defs = useAttributeDefs();
  return [
    ...QUOTE,
    ...COLUMNS.filter((column) => HOLDING.has(column.id)).map(held),
    ...recordColumns<WatchRow, WatchCtx>(),
    ...attributeColumns<WatchRow, WatchCtx>(defs.data ?? []),
  ];
}
