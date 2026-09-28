import { msg } from "@lingui/core/macro";
import { Money, Percent, Quantity } from "../../ui";
import { formatDay, toneClass, toNumber } from "../../../lib/format";
import { DIVIDEND_FREQUENCY_LABELS } from "../../../lib/kinds";
import type { PositionRow } from "../../../lib/types";
import { QuoteSpark } from "../QuoteSpark";
import type { CellCtx, Column } from "./types";
import { dash } from "./cells";
import { costColumns } from "./cost";
import { PERIOD_COLUMNS } from "./period";
import { recordColumns } from "./record";

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
  ...PERIOD_COLUMNS,
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
