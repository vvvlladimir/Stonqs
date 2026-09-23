/** Return, risk and benchmark over one period. */

import type { GrowthSeries, StatSeries, ValueSeries } from "./periods";
import type { DateString, MoneyString } from "./primitives";
import type { ChargeSummary } from "./reports";
import type { TradingVolume } from "./transactions";
export interface PeriodReturn {
  from: DateString;
  to: DateString;
  twr: MoneyString;
}

/** The money side of a period, next to the return side a TWR gives. */
export interface PeriodSummary {
  start_value_base: MoneyString;
  end_value_base: MoneyString;
  /** Net external flow; `+` is money brought in. */
  net_flow_base: MoneyString;
  /** End minus start: the change a statement shows, deposits included. */
  absolute_change_base: MoneyString;
  /** The change with the flows taken out — what was earned. */
  delta_base: MoneyString;
  /** Money committed by the end of the period. */
  invested_capital_base: MoneyString;
  /** Capital weighted by the time it was invested; the denominator of every cost rate. */
  average_capital_base: MoneyString;
}

export interface PerformanceData {
  from: DateString;
  to: DateString;
  base_currency: string;
  twr: MoneyString;
  /** `twr` as a yearly rate; `null` for a period shorter than a day. */
  twr_annualized: MoneyString | null;
  xirr: MoneyString | null;
  summary: PeriodSummary;
  /** Paid in the period, trade commissions included — unlike the charges report. */
  costs: ChargeSummary;
  fee_rate: MoneyString | null;
  tax_rate: MoneyString | null;
  /** Bought and sold in the period, and that volume over the capital at work. */
  volume: TradingVolume;
  turnover_rate: MoneyString | null;
  /** The period's highest value; `null` for a portfolio that never held anything. */
  peak: Peak | null;
  series: ValueSeries;
  growth: GrowthSeries;
  monthly_returns: PeriodReturn[];
  annual_returns: PeriodReturn[];
}

/** Calendar granularity the calculation sheet is split by. */
export type SheetPeriod = "DAY" | "WEEK" | "MONTH" | "QUARTER" | "YEAR";

/** One chunk of the calculation sheet: `end = start + flow + delta`, and what made the delta. */
export interface CalculationRow {
  from: DateString;
  to: DateString;
  /** The previous row's close; for the first row, the value before the window opened. */
  start_value_base: MoneyString;
  external_flow_base: MoneyString;
  /** Dividends and interest, gross — the tax withheld from them sits in `costs`. */
  income_base: MoneyString;
  costs: ChargeSummary;
  /** Fees and taxes added up, summed in the host so the UI never adds money strings. */
  costs_base: MoneyString;
  /** What is left of the change once income and costs are named. */
  market_change_base: MoneyString;
  delta_base: MoneyString;
  end_value_base: MoneyString;
  twr: MoneyString;
  /** The chunks so far, chained; the last row's value is the period's own return. */
  cumulative_twr: MoneyString;
}

export interface CalculationSheet {
  base_currency: string;
  period: SheetPeriod;
  rows: CalculationRow[];
  total: PeriodSummary;
  twr: MoneyString;
}

export interface Drawdown {
  peak: DateString;
  trough: DateString;
  recovered: DateString | null;
  /** Drawdown fraction; `-0.35` means -35%. */
  depth: number;
}

export interface RiskMetrics {
  days: number;
  volatility: number;
  semi_deviation: number;
  max_drawdown: Drawdown | null;
  /** Calendar days the deepest episode lasted, counted to the period end while underwater. */
  max_drawdown_days: number | null;
  /** Longest episode by duration, which is rarely the deepest one. */
  longest_drawdown: Drawdown | null;
  longest_drawdown_days: number | null;
  /** Distance from the peak right now; `0` means the period ends at one. */
  current_drawdown: number;
  /** The peak not yet climbed back to. */
  current_drawdown_since: DateString | null;
  sharpe: number | null;
  annualized_return: number;
  best_day: [DateString, number] | null;
  worst_day: [DateString, number] | null;
  positive_days_share: number;
}

/** Complete payload for the risk screen. */
export interface RiskReport {
  metrics: RiskMetrics;
  window_days: number;
  returns: StatSeries;
  rolling_volatility: StatSeries;
  drawdown: StatSeries;
  episodes: Drawdown[];
}

export interface BenchmarkComparison {
  from: DateString;
  to: DateString;
  portfolio_twr: MoneyString;
  benchmark_twr: MoneyString;
  excess: MoneyString;
}

// Allocation, taxonomies, and targets

/** Highest point of a series and where the last point stands against it. */
export interface Peak {
  date: DateString;
  value: MoneyString;
  current: MoneyString;
  /** `current / peak - 1`: zero at the high, negative below it. */
  distance: MoneyString | null;
  days_since: number;
}

/** A nominal return restated in the money of the period's first day. */
export interface RealReturn {
  /** ISO 3166-1 alpha-2, or `EA` / `EU` for the aggregates. The name is written here. */
  region: string;
  from: DateString;
  /** Where the index stops, which may be earlier than the period asked for. */
  to: DateString;
  /** What one unit of money at `from` costs at `to`: `1.04` is 4% of inflation. */
  factor: MoneyString;
  /** The same thing as a rate — `factor - 1`, computed by the host. */
  inflation: MoneyString;
  nominal: MoneyString;
  real: MoneyString;
  real_annualized: MoneyString | null;
}

/** A period's returns with inflation taken out; absent when no region is set. */
export interface RealPerformance {
  twr: RealReturn;
  /** Money-weighted, each flow deflated at its own date. */
  xirr: MoneyString | null;
}

/** What the app knows about the portfolio's price index. Codes only, never country names. */
export interface InflationStatus {
  region: string | null;
  source: string | null;
  /** First day of the last month a level is stored for. */
  published_through: DateString | null;
  /** Every region this build can fetch, for the picker. */
  regions: string[];
}
