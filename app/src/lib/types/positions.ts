/** What is held, what it is worth, and what it has returned. */

import type { DateString, MoneyString } from "./primitives";
import type { DividendFrequency } from "./securities";
export interface PositionValuation {
  security_id: string;
  /** Quote currency for price and market value. */
  currency: string;
  /** Broker settlement currency used for cost basis. */
  cost_currency: string;
  quantity: MoneyString;
  price: MoneyString;
  fx_rate: MoneyString;
  /** Settlement-currency rate on the same date; the currency split uses this one. */
  cost_fx_rate: MoneyString;
  market_value: MoneyString;
  market_value_base: MoneyString;
  cost_basis: MoneyString;
  cost_basis_base: MoneyString;
  unrealized_pnl_base: MoneyString;
  /** The exchange rate's share of the unrealized result. */
  currency_gain_base: MoneyString;
  realized_pnl_base: MoneyString;
  /** Quantity step inferred from trades, or null when unavailable. */
  observed_quantity_step: MoneyString | null;
}

export interface PortfolioValuation {
  date: DateString;
  base_currency: string;
  positions: PositionValuation[];
  securities_value_base: MoneyString;
  cash_base: MoneyString;
  total_value_base: MoneyString;
  cost_basis_base: MoneyString;
  unrealized_pnl_base: MoneyString;
  currency_gain_base: MoneyString;
  realized_pnl_base: MoneyString;
  realized_currency_gain_base: MoneyString;
  dividends_base: MoneyString;
  interest_base: MoneyString;
  fees_base: MoneyString;
  taxes_base: MoneyString;
}

export interface Lot {
  acquired_at: DateString;
  quantity: MoneyString;
  cost_per_unit: MoneyString;
  /** Unit cost in base currency at the acquisition-date FX rate. */
  cost_per_unit_base: MoneyString;
}

export interface PositionRow {
  security_id: string;
  symbol: string;
  name: string;
  /** Quote currency for `price`. */
  currency: string;
  /** Settlement currency for lot cost. */
  cost_currency: string;
  quantity: MoneyString;
  price: MoneyString;
  fx_rate: MoneyString;
  market_value_base: MoneyString;
  cost_basis_base: MoneyString;
  unrealized_pnl_base: MoneyString;
  /** The exchange rate's share of the unrealized result, and the instrument's own. */
  currency_gain_base: MoneyString;
  instrument_gain_base: MoneyString;
  realized_pnl_base: MoneyString;
  dividends_base: MoneyString;
  /** Payment schedule inferred from this instrument's own history. */
  dividend_frequency: DividendFrequency;
  dividend_payments: number;
  dividend_last: DateString | null;
  dividend_year_base: MoneyString;
  /** Last year's payments over today's value, and over the cost basis. */
  dividend_yield: MoneyString | null;
  yield_on_cost: MoneyString | null;
  /** Highest stored close in the quote currency, and how far below it the price stands. */
  ath_price: MoneyString | null;
  ath_date: DateString | null;
  ath_distance: MoneyString | null;
  /** Position weight; `"0.25"` means 25%. */
  weight: MoneyString;
  /** Previous quote close, or null when unavailable. */
  previous_price: MoneyString | null;
  /** Last trading-day change in base currency and as a fraction. */
  day_change_base: MoneyString | null;
  day_change: MoneyString | null;
  /** Security quantity held by each account. */
  accounts: PositionAccount[];
  lots: Lot[];
}

export interface PositionsData {
  date: DateString;
  base_currency: string;
  total_value_base: MoneyString;
  /** Aggregate last trading-day change. */
  day_change_base: MoneyString;
  rows: PositionRow[];
}

export interface PositionAccount {
  account_id: string;
  account_name: string;
  quantity: MoneyString;
}

export interface PositionReturn {
  twr: MoneyString | null;
  xirr: MoneyString | null;
}

/** Position-return row computed in one core pass. */
export interface PositionReturnRow {
  security_id: string;
  symbol: string;
  name: string;
  twr: MoneyString | null;
  /** The same return per year; `null` whenever `twr` is. */
  twr_annualized: MoneyString | null;
  xirr: MoneyString | null;
  /** Period earnings in base currency net of added capital. */
  pnl_base: MoneyString;
  /** Earnings against this position's own capital at work; `null` when it had none. */
  absolute_performance: MoneyString | null;
  /** Paid on this instrument in the period, trade commissions included. */
  fees_base: MoneyString;
  taxes_base: MoneyString;
  /** Contribution to portfolio return; `0.1` means +10 percentage points. */
  contribution: MoneyString;
  /** Risk of the position's own daily returns in the period; `null` with fewer than two days. */
  risk: PositionRisk | null;
}

/** Statistics, not money: plain numbers like the portfolio's `RiskMetrics`. */
export interface PositionRisk {
  volatility: number;
  semi_deviation: number;
  /** Depth of the deepest drawdown, `-0.2` for −20%; `0` when the position never fell. */
  max_drawdown: number;
  max_drawdown_days: number | null;
}

// Transactions

/** Purchase value and result of one instrument under one cost-basis method. */
export interface CostBasisFigures {
  /** In the currency the shares were paid for. */
  cost_basis: MoneyString;
  cost_basis_base: MoneyString;
  cost_per_unit: MoneyString;
  cost_per_unit_base: MoneyString;
  unrealized_pnl_base: MoneyString;
  realized_pnl_base: MoneyString;
}

/** One instrument read under both methods against one market value. */
export interface PositionCostRow {
  security_id: string;
  symbol: string;
  name: string;
  cost_currency: string;
  quantity: MoneyString;
  market_value_base: MoneyString;
  fifo: CostBasisFigures;
  average: CostBasisFigures;
}

export interface PositionCostData {
  date: DateString;
  base_currency: string;
  rows: PositionCostRow[];
}
