/** Every reading over a date or a period. */

import { useQueries, useQuery } from "@tanstack/react-query";
import { api, type Source } from "../api";
import type {
  DateString,
  PaymentPeriod,
  PeriodRange,
  SheetPeriod,
  TradeGrouping,
  TransactionKind,
} from "../types";
import { keys } from "./keys";

export function usePositions(date: DateString, source?: Source) {
  return useQuery({ queryKey: keys.positions(date, source), queryFn: () => api.positionsAt(date, source) });
}

/** Purchase value under both cost-basis methods. It costs a second holdings pass in the host,
 * so it stays idle until a screen actually shows one of those columns. */
export function usePositionsCostBasis(date: DateString | null, source?: Source) {
  return useQuery({
    queryKey: keys.positionsCostBasis(date ?? undefined, source),
    queryFn: () => api.positionsCostBasis(date!, source),
    enabled: date !== null,
  });
}

export function useDashboard(date: DateString, source?: Source) {
  return useQuery({
    queryKey: keys.dashboard(date, source),
    queryFn: () => api.dashboardSummary(date, source),
  });
}

export function useReports(range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.reports(range?.from, range?.to, source),
    queryFn: () => api.reportsSummary(range!.from, range!.to, source),
    enabled: range !== undefined,
  });
}

export function usePerformance(range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.performance(range?.from, range?.to, source),
    queryFn: () => api.performanceSummary(range!.from, range!.to, source),
    enabled: range !== undefined,
  });
}

/** The calculation sheet of the same period the performance screen is showing. */
export function usePerformanceBreakdown(
  range: PeriodRange | undefined,
  period: SheetPeriod,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.performanceBreakdown(range?.from, range?.to, period, source),
    queryFn: () => api.performanceBreakdown(range!.from, range!.to, period, source),
    enabled: range !== undefined,
  });
}

/** Dividends expected over the next `months` months; the window starts today, so no period. */
export function useExpectedDividends(months: number, source?: Source) {
  return useQuery({
    queryKey: keys.expectedDividends(months, source),
    queryFn: () => api.dividendsExpected(months, source),
  });
}

/** The payments grid over a window; the column width is part of the key. */
export function usePayments(range: PeriodRange | undefined, period: PaymentPeriod, source?: Source) {
  return useQuery({
    queryKey: keys.payments(range?.from, range?.to, period, source),
    queryFn: () => api.paymentsGrid(range!.from, range!.to, period, source),
    enabled: range !== undefined,
  });
}

/** `by` defaults to one trade per position, which is what a dashboard tile counts. */
export function useTrades(range: PeriodRange | undefined, by: TradeGrouping = "POSITION", source?: Source) {
  return useQuery({
    queryKey: keys.trades(range?.from, range?.to, by, source),
    queryFn: () => api.tradesSummary(range!.from, range!.to, by, source),
    enabled: range !== undefined,
  });
}

/** Rate and window stay strings: they are the key the Risk screen's controls produce. */
export function useRisk(
  range: PeriodRange | undefined,
  riskFreePercent: string,
  windowDays: string,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.risk(range?.from, range?.to, riskFreePercent, windowDays, source),
    queryFn: () =>
      api.riskReport(
        range!.from,
        range!.to,
        (Number(riskFreePercent) || 0) / 100,
        Number(windowDays),
        source,
      ),
    enabled: range !== undefined,
  });
}

export function useIncome(
  from: DateString,
  to: DateString,
  kind: TransactionKind | null = null,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.income(from, to, kind, source),
    queryFn: () => api.incomeSummary(from, to, kind, source),
  });
}

/** The same window split by one tree; a null id keeps the query idle. */
export function useIncomeTaxonomy(
  taxonomyId: string | null,
  range: PeriodRange | undefined,
  kind: TransactionKind | null = null,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.incomeTaxonomy(taxonomyId ?? undefined, range?.from, range?.to, kind, source),
    queryFn: () => api.incomeTaxonomy(taxonomyId!, range!.from, range!.to, kind, source),
    enabled: taxonomyId !== null && range !== undefined,
  });
}

/** Same window as `useIncome`, taken from a period range. */
export function useIncomeOver(
  range: PeriodRange | undefined,
  kind: TransactionKind | null = null,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.income(range?.from, range?.to, kind, source),
    queryFn: () => api.incomeSummary(range!.from, range!.to, kind, source),
    enabled: range !== undefined,
  });
}

/** Region, source and how far the stored index reaches. */
export function useInflationStatus() {
  return useQuery({ queryKey: keys.inflationStatus(), queryFn: api.inflationStatus });
}

/** The period's returns with inflation taken out; `null` while no region is set. */
export function useRealPerformance(range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.realPerformance(range?.from, range?.to, source),
    queryFn: () => api.realPerformance(range!.from, range!.to, source),
    enabled: range !== undefined,
  });
}

/** The cost of money over the period, shaped like a benchmark so the chart draws one more line. */
export function useInflationSeries(range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.inflationSeries(range?.from, range?.to, source),
    queryFn: () => api.inflationSeries(range!.from, range!.to, source),
    enabled: range !== undefined,
  });
}

export function useBenchmark(securityId: string, range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.benchmark(securityId, range?.from, range?.to, source),
    queryFn: () => api.benchmarkSeries(securityId, range!.from, range!.to, source),
    enabled: range !== undefined && securityId !== "",
  });
}

/** Several benchmarks at once, each under the same key `useBenchmark` gives it, so a tile with
 * three lines shares the cache with a tile holding any one of them. */
export function useBenchmarks(securityIds: string[], range: PeriodRange | undefined, source?: Source) {
  return useQueries({
    queries: securityIds.map((securityId) => ({
      queryKey: keys.benchmark(securityId, range?.from, range?.to, source),
      queryFn: () => api.benchmarkSeries(securityId, range!.from, range!.to, source),
      enabled: range !== undefined && securityId !== "",
    })),
  });
}

export function useBenchmarkCompare(securityId: string, range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.benchmarkCompare(securityId, range?.from, range?.to, source),
    queryFn: () => api.benchmarkCompare(securityId, range!.from, range!.to, source),
    enabled: range !== undefined && securityId !== "",
  });
}

/** All position returns in one history pass, not one query per security. */
export function usePositionReturns(range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.positionReturns(range?.from, range?.to, source),
    queryFn: () => api.positionReturns(range!.from, range!.to, source),
    enabled: range !== undefined,
  });
}

/** An instrument with no position has no window to measure over, so an absent `from` disables it. */
export function usePositionReturn(securityId: string, from: DateString | null, to: DateString) {
  return useQuery({
    queryKey: keys.positionReturn(securityId, from ?? undefined, to),
    queryFn: () => api.positionReturn(securityId, from!, to),
    enabled: from !== null,
  });
}
