import { msg } from "@lingui/core/macro";
import { Money, Num, Percent, Rate, Stat } from "../../../components/ui";
import { formatDay } from "../../../lib/format";
import {
  ContributionMetric,
  CostRate,
  DayMetric,
  DrawdownDays,
  FromPerformance,
  FromRisk,
  FromSummary,
  FromTrades,
  FromValuation,
  IncomeMetric,
  PeakMetric,
  TurnoverMetric,
} from "./value";
import type { MetricDef } from "./model";

/** The metric catalog: every number a `metric` widget can show. */
export const METRICS: Record<string, MetricDef> = {
  value: {
    label: msg`Value`,
    tip: msg`Instruments at their last quote plus cash, in the reporting currency.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="total_value_base" />,
  },
  securities: {
    label: msg`Securities`,
    tip: msg`The value of open positions without cash balances.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="securities_value_base" />,
  },
  cash: {
    label: msg`Cash`,
    tip: msg`The cash balances of the selection, converted to the reporting currency.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="cash_base" />,
  },
  invested: {
    label: msg`Cost basis`,
    tip: msg`What the open positions cost: the trade price plus the purchase commission.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="cost_basis_base" />,
  },
  unrealized: {
    label: msg`Unrealized P/L`,
    tip: msg`The value of open positions minus their cost basis.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="unrealized_pnl_base" signed />,
  },
  realized: {
    label: msg`Realized P/L`,
    tip: msg`The result of closed trades over the whole history, by the portfolio's cost-basis method.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="realized_pnl_base" signed />,
  },
  dividends: {
    label: msg`Dividends`,
    tip: msg`Payments from instruments over the whole history, before withholding tax.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="dividends_base" />,
  },
  contribution: {
    label: msg`Contributions`,
    tip: msg`What the active plans add up to in an average month, read off the coming year.`,
    Render: (ctx) => <ContributionMetric ctx={ctx} />,
  },
  interest: {
    label: msg`Interest`,
    tip: msg`Interest on cash balances over the whole history.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="interest_base" />,
  },
  fees: {
    label: msg`Fees`,
    tip: msg`Standalone "Fee" transactions: a commission inside a trade already sits in the cost basis.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="fees_base" />,
  },
  taxes: {
    label: msg`Taxes`,
    tip: msg`Standalone "Tax" transactions: dividend tax sits next to its own payment.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="taxes_base" />,
  },
  day: {
    label: msg`Today`,
    tip: msg`The change against the previous day with a quote, not against yesterday's calendar date.`,
    Render: DayMetric,
  },
  twr: {
    label: msg`Return (TWR)`,
    tip: msg`A return independent of deposits: how the instruments themselves did.`,
    periodic: true,
    Render: (ctx) => <FromPerformance ctx={ctx} field="twr" />,
  },
  irr: {
    label: msg`Return (IRR)`,
    tip: msg`The annual return on the money invested, accounting for the dates of deposits.`,
    periodic: true,
    Render: (ctx) => <FromPerformance ctx={ctx} field="xirr" />,
  },
  twrann: {
    label: msg`Return a year (TWR)`,
    tip: msg`The period return restated as a yearly rate, so two periods of different lengths can be compared.`,
    periodic: true,
    Render: (ctx) => <FromPerformance ctx={ctx} field="twr_annualized" />,
  },
  delta: {
    label: msg`Earned over the period`,
    tip: msg`The change in value with deposits and withdrawals taken out: a transfer in is not a result.`,
    periodic: true,
    Render: (ctx) => <FromSummary ctx={ctx} field="delta_base" signed />,
  },
  change: {
    label: msg`Change in value`,
    tip: msg`End of period minus start, deposits included — the figure a statement shows.`,
    periodic: true,
    Render: (ctx) => <FromSummary ctx={ctx} field="absolute_change_base" signed />,
  },
  netinvested: {
    label: msg`Invested capital`,
    tip: msg`What the portfolio was worth when the period opened plus everything paid in since.`,
    periodic: true,
    Render: (ctx) => <FromSummary ctx={ctx} field="invested_capital_base" />,
  },
  feerate: {
    label: msg`Fee rate`,
    tip: msg`Every fee paid in the period against the capital at work, trade commissions included.`,
    periodic: true,
    Render: (ctx) => <CostRate ctx={ctx} field="fee_rate" />,
  },
  taxrate: {
    label: msg`Tax rate`,
    tip: msg`Tax paid in the period against the capital at work, dividend withholding included.`,
    periodic: true,
    Render: (ctx) => <CostRate ctx={ctx} field="tax_rate" />,
  },
  income: {
    label: msg`Income for the period`,
    tip: msg`Dividends and interest for the period, net of withheld taxes.`,
    periodic: true,
    Render: IncomeMetric,
  },
  vol: {
    label: msg`Volatility`,
    tip: msg`The annualized standard deviation of daily returns, over trading days.`,
    periodic: true,
    Render: (ctx) => <FromRisk ctx={ctx} pick={(m) => <Rate value={m.volatility} />} />,
  },
  sharpe: {
    label: msg`Sharpe`,
    tip: msg`Return above the risk-free rate per unit of volatility.`,
    periodic: true,
    Render: (ctx) => <FromRisk ctx={ctx} pick={(m) => <Stat value={m.sharpe} />} />,
  },
  curdd: {
    label: msg`Current drawdown`,
    tip: msg`How far below the last peak the portfolio stands now — not the depth of the hole it is climbing out of.`,
    periodic: true,
    Render: (ctx) => (
      <FromRisk
        ctx={ctx}
        tone="neg"
        pick={(m) => <Rate value={m.current_drawdown} />}
        foot={(m, i18n) =>
          m.current_drawdown_since
            ? i18n._(msg`under the peak of ${formatDay(m.current_drawdown_since)}`)
            : i18n._(msg`at its peak`)
        }
      />
    ),
  },
  ddur: {
    label: msg`Longest drawdown`,
    tip: msg`Days between a peak and the return to it. Rarely the deepest episode: a shallow hole that never fills is the one that costs years.`,
    periodic: true,
    Render: (ctx) => <DrawdownDays ctx={ctx} pick={(m) => m.longest_drawdown_days} />,
  },
  fxgain: {
    label: msg`Of the result, FX`,
    tip: msg`The part of the unrealized result the exchange rate made rather than the instruments — zero while everything is held in the reporting currency.`,
    Render: (ctx) => <FromValuation ctx={ctx} field="currency_gain_base" signed />,
  },
  ath: {
    label: msg`Period high`,
    tip: msg`The highest the portfolio was worth inside the period, and how far under it the period ends. Measured on value, so a deposit does raise it.`,
    periodic: true,
    Render: (ctx) => <PeakMetric ctx={ctx} />,
  },
  turnover: {
    label: msg`Turnover`,
    tip: msg`Everything bought and sold in the period against the capital at work: what the trading itself amounts to.`,
    periodic: true,
    Render: (ctx) => <TurnoverMetric ctx={ctx} />,
  },
  tradepnl: {
    label: msg`Result of closed trades`,
    tip: msg`Exit value minus entry value over the trades closed inside the period, commissions of both ends included.`,
    periodic: true,
    Render: (ctx) => (
      <FromTrades
        ctx={ctx}
        pick={(data) => <Money value={data.closed_stats.pnl_base} currency={data.base_currency} signed />}
        foot={(data) => <Num>{data.closed_stats.trades}</Num>}
      />
    ),
  },
  winrate: {
    label: msg`Hit rate`,
    tip: msg`The share of closed trades that ended above their entry value. It says nothing about size: ten small wins and one large loss still read as 91%.`,
    periodic: true,
    Render: (ctx) => (
      <FromTrades
        ctx={ctx}
        pick={(data) =>
          data.closed_stats.win_rate === null ? (
            "—"
          ) : (
            <Percent value={data.closed_stats.win_rate} digits={0} />
          )
        }
        foot={(data) => (
          <>
            <Num>{data.closed_stats.winners}</Num> / <Num>{data.closed_stats.trades}</Num>
          </>
        )}
      />
    ),
  },
  maxdd: {
    label: msg`Maximum drawdown`,
    tip: msg`The deepest fall from a previous peak, from chained returns.`,
    periodic: true,
    Render: (ctx) => (
      <FromRisk
        ctx={ctx}
        tone="neg"
        pick={(m) => (m.max_drawdown ? <Rate value={m.max_drawdown.depth} /> : "—")}
        foot={(m, i18n) =>
          m.max_drawdown ? i18n._(msg`trough ${formatDay(m.max_drawdown.trough)}`) : undefined
        }
      />
    ),
  },
};
