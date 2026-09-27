/** Reporting reads over a period: positions, performance, payments, risk, allocation, reports. */

import type {
  PaymentPeriod,
  PaymentsData,
  ExpectedDividendsData,
  Allocation,
  NodeMember,
  BenchmarkComparison,
  PerformanceData,
  CalculationSheet,
  SheetPeriod,
  PeriodRange,
  PeriodSettings,
  UserPeriod,
  PositionReturn,
  PositionReturnRow,
  PositionsData,
  PositionCostData,
  ReportsData,
  TradeGrouping,
  TradesData,
  IncomeData,
  TaxonomyIncomeData,
  RiskReport,
  GrowthSeries,
  InflationStatus,
  TransactionKind,
  DateString,
  RealPerformance,
} from "../types";
import { call, type Source } from "./core";

export const reportsApi = {
  /** The same positions' purchase value under both cost-basis methods. */
  positionsCostBasis: (date: DateString, source?: Source) =>
    call<PositionCostData>("positions_cost_basis", { date, source: source ?? null }),
  positionsAt: (date: DateString, source?: Source) =>
    call<PositionsData>("positions_at", { date, source: source ?? null }),
  positionReturn: (security_id: string, from: DateString, to: DateString, source?: Source) =>
    call<PositionReturn>("position_return", { securityId: security_id, from, to, source: source ?? null }),
  /** Returns and contributions for all positions in one history pass. */
  positionReturns: (from: DateString, to: DateString, source?: Source) =>
    call<PositionReturnRow[]>("position_returns", { from, to, source: source ?? null }),

  periodRanges: (as_of: DateString) => call<PeriodRange[]>("period_ranges", { asOf: as_of }),
  periodsGet: () => call<PeriodSettings>("periods_get"),
  periodSave: (period: UserPeriod) => call<PeriodSettings>("period_save", { period }),
  /** Deletes the user's own period; a shipped preset is only hidden. */
  periodDelete: (id: string) => call<PeriodSettings>("period_delete", { id }),
  periodsRestore: () => call<PeriodSettings>("periods_restore"),
  performanceSummary: (from: DateString, to: DateString, source?: Source) =>
    call<PerformanceData>("performance_summary", { from, to, source: source ?? null }),
  /** The calculation sheet: one row per calendar chunk of the same period. */
  performanceBreakdown: (from: DateString, to: DateString, period: SheetPeriod, source?: Source) =>
    call<CalculationSheet>("performance_breakdown", { from, to, period, source: source ?? null }),
  /** Writes the same sheet as a CSV where the user points. */
  performanceSheetSave: (from: DateString, to: DateString, period: SheetPeriod, path: string) =>
    call<void>("performance_sheet_save", { from, to, period, path }),
  /** The payments grid: every dated line of the window on one axis. */
  paymentsGrid: (from: DateString, to: DateString, period: PaymentPeriod, source?: Source) =>
    call<PaymentsData>("payments_grid", { from, to, period, source: source ?? null }),
  /** Dividends the open positions should pay over the next `months` months, at today's rates. */
  dividendsExpected: (months: number, source?: Source) =>
    call<ExpectedDividendsData>("dividends_expected", { months, source: source ?? null }),
  /** Open and closed trades of a window, with the turnover the trading produced. */
  tradesSummary: (from: DateString, to: DateString, by: TradeGrouping, source?: Source) =>
    call<TradesData>("trades_summary", { from, to, by, source: source ?? null }),
  /** Full risk report; rolling-volatility window is in trading days. */
  riskReport: (from: DateString, to: DateString, risk_free_rate: number, window_days = 63, source?: Source) =>
    call<RiskReport>("risk_report", {
      from,
      to,
      riskFreeRate: risk_free_rate,
      windowDays: window_days,
      source: source ?? null,
    }),
  benchmarkCompare: (security_id: string, from: DateString, to: DateString, source?: Source) =>
    call<BenchmarkComparison>("benchmark_compare", {
      securityId: security_id,
      from,
      to,
      source: source ?? null,
    }),
  /** Benchmark growth aligned to the portfolio dates. */
  benchmarkSeries: (security_id: string, from: DateString, to: DateString, source?: Source) =>
    call<GrowthSeries>("benchmark_series", { securityId: security_id, from, to, source: source ?? null }),

  /** Region, source and how far the stored index reaches. */
  inflationStatus: () => call<InflationStatus>("inflation_status"),

  /** Points the portfolio at a price-index region, or switches real returns off with `null`. */
  inflationRegionSet: (region: string | null) => call<void>("inflation_region_set", { region }),

  /** The period's returns with inflation taken out; `null` when no region is set. */
  realPerformance: (from: DateString, to: DateString, source?: Source) =>
    call<RealPerformance | null>("real_performance", { from, to, source: source ?? null }),

  /** Cost of one unit of money across the period, based at its first day. */
  inflationSeries: (from: DateString, to: DateString, source?: Source) =>
    call<GrowthSeries | null>("inflation_series", { from, to, source: source ?? null }),

  allocation: (cut: string, taxonomy_id: string | null, date: DateString, source?: Source) =>
    call<Allocation>("allocation", { cut, taxonomyId: taxonomy_id, date, source: source ?? null }),

  /** Members and assigned portions for one allocation level. */
  allocationMembers: (taxonomy_id: string, node_id: string | null, date: DateString, source?: Source) =>
    call<NodeMember[]>("allocation_members", {
      taxonomyId: taxonomy_id,
      nodeId: node_id,
      date,
      source: source ?? null,
    }),

  /** Full category tree for the full-depth hierarchy view. */
  allocationTree: (taxonomy_id: string, date: DateString, source?: Source) =>
    call<Allocation>("allocation_tree", { taxonomyId: taxonomy_id, date, source: source ?? null }),

  reportsSummary: (from: DateString, to: DateString, source?: Source) =>
    call<ReportsData>("reports_summary", { from, to, source: source ?? null }),
  /** Income events and core-computed breakdowns for a period. */
  /** `kind` narrows the report to one income kind; null keeps every kind. */
  incomeSummary: (from: DateString, to: DateString, kind: TransactionKind | null = null, source?: Source) =>
    call<IncomeData>("income_summary", { from, to, kind, source: source ?? null }),
  /** The same period's income split by one classification tree. */
  incomeTaxonomy: (
    taxonomyId: string,
    from: DateString,
    to: DateString,
    kind: TransactionKind | null = null,
    source?: Source,
  ) =>
    call<TaxonomyIncomeData>("income_taxonomy", {
      taxonomyId,
      from,
      to,
      kind,
      source: source ?? null,
    }),
  /** `section` names one table of the report ("gains.detail", "charges.account", …). */
  reportSave: (section: string, from: DateString, to: DateString, path: string, footer: string) =>
    call<void>("report_save", { section, from, to, path, footer }),
};
