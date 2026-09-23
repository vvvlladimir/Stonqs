import { useCallback } from "react";
import { useQueries, useQuery, useQueryClient, type QueryKey } from "@tanstack/react-query";
import { api, today, type Source } from "./api";
import type {
  DataChangeKind,
  DateString,
  PaymentPeriod,
  PeriodRange,
  SheetPeriod,
  TransactionFilter,
  TransactionKind,
} from "./types";

/**
 * The data layer: one place where a query key is spelled and one place that says
 * what a write invalidates. A screen calls a hook, never `useQuery` with a key.
 */

/**
 * Drops trailing `undefined` arguments, so `keys.positions()` is a prefix of
 * `keys.positions(date)` — invalidating the prefix covers every date.
 * `null` is a real argument (no taxonomy, every kind) and stays in the key.
 */
function key(...parts: unknown[]): QueryKey {
  while (parts.length > 0 && parts[parts.length - 1] === undefined) parts.pop();
  return parts;
}

export const keys = {
  status: () => key("status"),
  settings: () => key("settings"),
  profiles: () => key("profiles"),
  portfolio: () => key("portfolio"),
  aiKeyStatus: (provider?: string) => key("ai-key-status", provider),
  aiChats: () => key("ai-chats"),
  aiMessages: (chatId?: string) => key("ai-messages", chatId),
  aiGrants: (chatId?: string) => key("ai-grants", chatId),
  aiUsage: () => key("ai-usage"),
  aiProviders: () => key("ai-providers"),
  aiModels: (provider?: string) => key("ai-models", provider),

  accounts: () => key("accounts"),
  accountGroups: () => key("account-groups"),
  accountsTotal: () => key("accounts-total"),
  scope: () => key("scope"),

  securities: () => key("securities"),
  attributeDefs: () => key("attribute-defs"),
  taxonomyGrouping: (attributeId?: string, into?: string) => key("taxonomy-grouping", attributeId, into),
  quoteProviders: () => key("quote-providers"),
  marketSources: () => key("market-sources"),
  marketCustom: () => key("market-custom"),
  listings: (securityId?: string) => key("listings", securityId),
  quotes: (securityId?: string, from?: DateString, to?: DateString) => key("quotes", securityId, from, to),
  coverage: () => key("coverage"),
  corporateActions: (securityId?: string) => key("corporate-actions", securityId),
  alerts: (securityId?: string) => key("alerts", securityId),
  alertCrossings: (limit?: number) => key("alert-crossings", limit),
  alertsUnseen: () => key("alerts-unseen"),
  securityEvents: (securityId?: string) => key("security-events", securityId),
  refreshStatus: () => key("refresh-status"),

  positions: (date?: DateString, source?: Source) => key("positions", date, source ?? undefined),
  dashboard: (date?: DateString, source?: Source) => key("dashboard", date, source ?? undefined),
  transactions: (filter?: TransactionFilter) => key("transactions", filter),
  transferSuggestions: () => key("transferSuggestions"),
  reports: (from?: DateString, to?: DateString, source?: Source) =>
    key("reports", from, to, source ?? undefined),

  periods: (asOf?: DateString) => key("periods", asOf),
  periodSettings: () => key("period-settings"),
  performance: (from?: DateString, to?: DateString, source?: Source) =>
    key("performance", from, to, source ?? undefined),
  performanceBreakdown: (from?: DateString, to?: DateString, period?: SheetPeriod, source?: Source) =>
    key("performance-breakdown", from, to, period, source ?? undefined),
  goals: (date?: DateString) => key("goals", date),
  limits: (date?: DateString) => key("limits", date),
  trades: (from?: DateString, to?: DateString, source?: Source) =>
    key("trades", from, to, source ?? undefined),
  payments: (from?: DateString, to?: DateString, period?: PaymentPeriod, source?: Source) =>
    key("payments", from, to, period, source ?? undefined),
  expectedDividends: (months?: number, source?: Source) =>
    key("expected-dividends", months, source ?? undefined),
  risk: (from?: DateString, to?: DateString, riskFree?: string, windowDays?: string, source?: Source) =>
    key("risk", from, to, riskFree, windowDays, source ?? undefined),
  income: (from?: DateString, to?: DateString, kind?: TransactionKind | null, source?: Source) =>
    key("income", from, to, kind, source ?? undefined),
  incomeTaxonomy: (
    taxonomyId?: string,
    from?: DateString,
    to?: DateString,
    kind?: TransactionKind | null,
    source?: Source,
  ) => key("income-taxonomy", taxonomyId, from, to, kind, source ?? undefined),
  inflationStatus: () => key("inflation-status"),
  realPerformance: (from?: DateString, to?: DateString, source?: Source) =>
    key("real-performance", from, to, source ?? undefined),
  inflationSeries: (from?: DateString, to?: DateString, source?: Source) =>
    key("inflation-series", from, to, source ?? undefined),
  benchmark: (securityId?: string, from?: DateString, to?: DateString, source?: Source) =>
    key("benchmark", securityId, from, to, source ?? undefined),
  benchmarkCompare: (securityId?: string, from?: DateString, to?: DateString, source?: Source) =>
    key("benchmark-compare", securityId, from, to, source ?? undefined),
  positionsCostBasis: (date?: DateString, source?: Source) =>
    key("positions-cost-basis", date, source ?? undefined),
  positionReturns: (from?: DateString, to?: DateString, source?: Source) =>
    key("position-returns", from, to, source ?? undefined),
  positionReturn: (securityId?: string, from?: DateString, to?: DateString, source?: Source) =>
    key("position-return", securityId, from, to, source ?? undefined),

  taxonomies: () => key("taxonomies"),
  allocation: (cut?: string, taxonomyId?: string | null, date?: DateString, source?: Source) =>
    key("allocation", cut, taxonomyId, date, source ?? undefined),
  allocationMembers: (taxonomyId?: string, nodeId?: string | null, date?: DateString, source?: Source) =>
    key("allocation-members", taxonomyId, nodeId, date, source ?? undefined),
  allocationTree: (taxonomyId?: string, date?: DateString, source?: Source) =>
    key("allocation-tree", taxonomyId, date, source ?? undefined),

  targets: () => key("targets"),
  rebalance: (
    targetId?: string,
    date?: DateString,
    cash?: string | null,
    allowSell?: boolean,
    source?: Source,
  ) => key("rebalance", targetId, date, cash, allowSell, source ?? undefined),

  importTemplates: () => key("import-templates"),

  plans: () => key("plans"),
  planDue: (planId?: string) => key("plan-due", planId),
  planProjection: (months?: number) => key("plan-projection", months),
  fire: (spending?: string, rate?: string, ret?: string, contribution?: string | null) =>
    key("fire", spending, rate, ret, contribution ?? undefined),

  watchlists: () => key("watchlists"),
  watchlistRows: (id?: string, from?: DateString, to?: DateString) => key("watchlist-rows", id, from, to),
};

// --- Queries -----------------------------------------------------------------
// A hook with a nullable id or an absent range stays disabled instead of making
// the caller repeat `enabled` and a non-null assertion.

export function useStatus() {
  return useQuery({ queryKey: keys.status(), queryFn: api.appStatus });
}

export function useProfiles() {
  return useQuery({ queryKey: keys.profiles(), queryFn: api.profilesList });
}

export function useSettings() {
  return useQuery({ queryKey: keys.settings(), queryFn: api.settingsGet });
}

/** Whether a key is saved for the given provider — never the key itself. */
export function useAiKeyStatus(provider: string) {
  return useQuery({ queryKey: keys.aiKeyStatus(provider), queryFn: () => api.aiKeyStatus(provider) });
}

export function useAiChats() {
  return useQuery({ queryKey: keys.aiChats(), queryFn: api.aiChatsList });
}

/** Without an open chat the hook stays disabled rather than listing nothing. */
export function useAiMessages(chatId: string | null) {
  return useQuery({
    queryKey: keys.aiMessages(chatId ?? undefined),
    queryFn: () => api.aiMessagesList(chatId!),
    enabled: chatId !== null,
  });
}

/** The providers this build can talk to. Compiled in, so it never goes stale while the app runs. */
export function useAiProviders() {
  return useQuery({ queryKey: keys.aiProviders(), queryFn: api.aiProvidersList, staleTime: Infinity });
}

/** What one provider offers. Asked once per provider per run: a model list is a network call,
 * and the answer does not change while the app is open. Keyed by provider, so a chat switched to
 * another one asks that catalogue rather than reusing the first's. */
export function useAiModels(provider: string | null, enabled: boolean) {
  return useQuery({
    queryKey: keys.aiModels(provider ?? undefined),
    queryFn: () => api.aiModelsList(provider!),
    enabled: enabled && provider !== null,
    staleTime: Infinity,
  });
}

/** What every model has cost so far. Refetched with the chat list: a finished turn moves both. */
export function useAiUsage() {
  return useQuery({ queryKey: keys.aiUsage(), queryFn: api.aiUsageTotals });
}

/** The tools this chat may already read without asking again. */
export function useAiGrants(chatId: string | null) {
  return useQuery({
    queryKey: keys.aiGrants(chatId ?? undefined),
    queryFn: () => api.aiGrantsList(chatId!),
    enabled: chatId !== null,
  });
}

export function usePortfolio() {
  return useQuery({ queryKey: keys.portfolio(), queryFn: api.portfolioGet });
}

export function useAccounts() {
  return useQuery({ queryKey: keys.accounts(), queryFn: api.accountsList });
}

export function useAccountGroups() {
  return useQuery({ queryKey: keys.accountGroups(), queryFn: api.accountGroupsList });
}

export function useAccountsTotal() {
  return useQuery({ queryKey: keys.accountsTotal(), queryFn: api.accountsTotal });
}

export function usePlans() {
  return useQuery({ queryKey: keys.plans(), queryFn: api.plansList });
}

/** Disabled without a plan: an id is what makes the question answerable. */
export function usePlanDue(planId: string | null) {
  return useQuery({
    queryKey: keys.planDue(planId ?? undefined),
    queryFn: () => api.planDue(planId!),
    enabled: planId !== null,
  });
}

export function usePlanProjection(months: number) {
  return useQuery({
    queryKey: keys.planProjection(months),
    queryFn: () => api.planProjection(months),
  });
}

/** The assumptions are the key: two tiles asking different questions are two queries. */
export function useFire(assumptions: {
  annual_spending: string;
  withdrawal_rate: string;
  expected_return: string;
  contribution: string | null;
}) {
  const { annual_spending, withdrawal_rate, expected_return, contribution } = assumptions;
  return useQuery({
    queryKey: keys.fire(annual_spending, withdrawal_rate, expected_return, contribution),
    queryFn: () => api.fireProjection(assumptions),
    enabled: annual_spending !== "" && withdrawal_rate !== "" && expected_return !== "",
  });
}

export function useWatchlists() {
  return useQuery({ queryKey: keys.watchlists(), queryFn: api.watchlistsList });
}

/** Disabled without a list or a period: both are what the rows are read over. */
export function useWatchlistRows(id: string | null, range: PeriodRange | undefined) {
  return useQuery({
    queryKey: keys.watchlistRows(id ?? undefined, range?.from, range?.to),
    queryFn: () => api.watchlistRows(id!, range!.from, range!.to),
    enabled: id !== null && range !== undefined,
  });
}

export function useScope() {
  return useQuery({ queryKey: keys.scope(), queryFn: api.scopeGet });
}

export function useSecurities() {
  return useQuery({ queryKey: keys.securities(), queryFn: api.securitiesList });
}

/** Splits of one instrument; without an id the hook stays disabled rather than listing all. */
export function useCorporateActions(securityId: string | null) {
  return useQuery({
    queryKey: keys.corporateActions(securityId ?? undefined),
    queryFn: () => api.corporateActionsList(securityId!),
    enabled: securityId !== null,
  });
}

/** Rules with their status: of one instrument, or of every instrument when the id is absent. */
/** The crossing log across every rule, newest first. */
export function useAlertCrossings(limit: number) {
  return useQuery({ queryKey: keys.alertCrossings(limit), queryFn: () => api.alertCrossingsList(limit) });
}

/** Crossings not looked at yet: the dot beside Alerts in the navigation. */
export function useAlertsUnseen() {
  return useQuery({ queryKey: keys.alertsUnseen(), queryFn: api.alertsUnseen });
}

export function useAlerts(securityId?: string) {
  return useQuery({
    queryKey: keys.alerts(securityId),
    queryFn: () => api.alertsList(securityId),
  });
}

/** Notes, dividends and splits: of one instrument, or of every instrument when the id is absent. */
export function useSecurityEvents(securityId?: string) {
  return useQuery({
    queryKey: keys.securityEvents(securityId),
    queryFn: () => api.securityEventsList(securityId),
  });
}

/** The instrument attributes the user defined; the values live on the security rows. */
export function useAttributeDefs() {
  return useQuery({ queryKey: keys.attributeDefs(), queryFn: api.attributeDefsList });
}

/** What grouping a tree by one attribute would do, before it is written. */
export function useTaxonomyGrouping(attributeId: string | null, into: string | null) {
  return useQuery({
    queryKey: keys.taxonomyGrouping(attributeId ?? undefined, into ?? undefined),
    queryFn: () => api.taxonomyGroupPreview(attributeId!, into),
    enabled: attributeId !== null,
  });
}

export function useQuoteProviders() {
  return useQuery({ queryKey: keys.quoteProviders(), queryFn: api.quoteProviders });
}

export function useMarketSources() {
  return useQuery({ queryKey: keys.marketSources(), queryFn: api.marketSources });
}

export function useMarketCustom() {
  return useQuery({ queryKey: keys.marketCustom(), queryFn: api.marketCustom });
}

/** Venue list for one security; the host may reach the network for it. */
export function useListings(securityId: string) {
  return useQuery({
    queryKey: keys.listings(securityId),
    queryFn: () => api.securityListings(securityId, false),
  });
}

/** Length of the quote window behind a sparkline; not a period preset (see ADR-0018). */
export const QUOTE_WINDOW_DAYS = 90;

/**
 * Quotes for the last `days` calendar days. The window is computed here so every caller
 * asking for the same one shares a key — and no screen does date arithmetic of its own.
 */
export function useQuoteWindow(securityId: string, days: number = QUOTE_WINDOW_DAYS) {
  const to = today();
  const from = daysBefore(to, days);
  return useQuery({
    queryKey: keys.quotes(securityId, from, to),
    queryFn: () => api.quotesRange(securityId, from, to),
  });
}

/** Subtract calendar days from an ISO date. */
function daysBefore(date: DateString, days: number): DateString {
  const [year, month, day] = date.split("-").map(Number);
  return new Date(Date.UTC(year, month - 1, day - days)).toISOString().slice(0, 10);
}

export function useCoverage() {
  return useQuery({ queryKey: keys.coverage(), queryFn: api.dataCoverage });
}

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

export function useTransactions(filter: TransactionFilter) {
  return useQuery({
    queryKey: keys.transactions(filter),
    queryFn: () => api.transactionsList(filter),
  });
}

/**
 * Moves that arrived as two unrelated rows. Read on demand — the import screen asks after a
 * write — rather than on every ledger render: it is a whole-portfolio scan, and nothing on the
 * screen is wrong while the answer is missing.
 */
export function useTransferSuggestions(enabled: boolean) {
  return useQuery({
    queryKey: keys.transferSuggestions(),
    queryFn: api.transferSuggestions,
    enabled,
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

/** Goals with their progress, read at `date`. Not scoped: a goal carries its own accounts. */
export function useGoals(date: DateString) {
  return useQuery({ queryKey: keys.goals(date), queryFn: () => api.goalsList(date) });
}

/** Contribution limits with what has been paid in over the limit year `date` falls in. */
export function useLimits(date: DateString) {
  return useQuery({ queryKey: keys.limits(date), queryFn: () => api.limitsList(date) });
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

export function useTrades(range: PeriodRange | undefined, source?: Source) {
  return useQuery({
    queryKey: keys.trades(range?.from, range?.to, source),
    queryFn: () => api.tradesSummary(range!.from, range!.to, source),
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

export function useTaxonomies() {
  return useQuery({ queryKey: keys.taxonomies(), queryFn: api.taxonomiesList });
}

/** `taxonomyId` null means the security breakdown, which needs no tree. */
export function useAllocation(cut: string, taxonomyId: string | null, date: DateString, source?: Source) {
  return useQuery({
    queryKey: keys.allocation(cut, taxonomyId, date, source),
    queryFn: () => api.allocation(cut, taxonomyId, date, source),
    enabled: cut !== "taxonomy" || taxonomyId !== null,
  });
}

export function useAllocationMembers(
  taxonomyId: string | null,
  nodeId: string | null,
  date: DateString,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.allocationMembers(taxonomyId ?? undefined, nodeId, date, source),
    queryFn: () => api.allocationMembers(taxonomyId!, nodeId, date, source),
    enabled: taxonomyId !== null,
  });
}

export function useAllocationTree(taxonomyId: string | null, date: DateString, source?: Source) {
  return useQuery({
    queryKey: keys.allocationTree(taxonomyId ?? undefined, date, source),
    queryFn: () => api.allocationTree(taxonomyId!, date, source),
    enabled: taxonomyId !== null,
  });
}

export function useTargets() {
  return useQuery({ queryKey: keys.targets(), queryFn: api.targetsList });
}

export function useRebalance(
  targetId: string | null,
  date: DateString,
  cash: string | null = null,
  allowSell = true,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.rebalance(targetId ?? undefined, date, cash, allowSell, source),
    queryFn: () => api.rebalancePlan(targetId!, date, cash, allowSell, source),
    enabled: targetId !== null,
  });
}

export function useImportTemplates() {
  return useQuery({ queryKey: keys.importTemplates(), queryFn: api.importTemplates });
}

// --- Invalidation ------------------------------------------------------------

/** Every key computed from transactions and prices — a report, not a stored list. */
const REPORTS: QueryKey[] = [
  keys.positions(),
  keys.dashboard(),
  keys.transactions(),
  keys.reports(),
  keys.periods(),
  keys.performance(),
  keys.performanceBreakdown(),
  keys.trades(),
  keys.payments(),
  keys.expectedDividends(),
  keys.risk(),
  keys.income(),
  keys.incomeTaxonomy(),
  keys.realPerformance(),
  keys.inflationSeries(),
  keys.benchmark(),
  keys.benchmarkCompare(),
  keys.positionsCostBasis(),
  keys.fire(),
  keys.positionReturns(),
  keys.positionReturn(),
  keys.allocation(),
  keys.allocationMembers(),
  keys.allocationTree(),
  keys.rebalance(),
  keys.accountsTotal(),
];

/** Rules and events of instruments. A fresh quote decides whether a limit fired. A watch row
 * carries the nearest level and the reported dividends, so it moves with them. */
const ALERTS: QueryKey[] = [
  keys.alerts(),
  keys.alertCrossings(),
  keys.alertsUnseen(),
  keys.securityEvents(),
  keys.watchlistRows(),
];

/** A plan and everything read off its schedule. Committing one also writes transactions. */
const PLANS: QueryKey[] = [keys.plans(), keys.planDue(), keys.planProjection(), keys.fire()];

/** A goal reads a valuation and a limit reads the ledger, so both move with either. */
const GOALS: QueryKey[] = [keys.goals(), keys.limits()];

/**
 * Stored lists that nonetheless carry computed fields: an account row holds its balance,
 * a security row its quote count and trade count. A write to one of them moves all three.
 */
const LISTS: QueryKey[] = [keys.accounts(), keys.accountGroups(), keys.securities()];

/** Taxonomy trees and everything derived from them; transactions do not touch these. */
const TREES: QueryKey[] = [
  keys.taxonomies(),
  keys.taxonomyGrouping(),
  keys.allocation(),
  keys.incomeTaxonomy(),
  keys.realPerformance(),
  keys.inflationSeries(),
  keys.allocationMembers(),
  keys.allocationTree(),
  keys.rebalance(),
];

/**
 * What a change invalidates, named after the `scope` the host puts on `data:changed`
 * (`app/src-tauri/src/events.rs`), so a mutation and the host event agree on one list.
 */
export const affects: Record<DataChangeKind, QueryKey[]> = {
  ai_chats: [keys.aiChats(), keys.aiGrants(), keys.aiUsage()],
  // An import writes transactions and may create securities; both rows carry counts.
  // Committing a plan writes transactions, so the occurrence it answered stops being due.
  transactions: [...LISTS, ...REPORTS, ...PLANS, ...GOALS, keys.transferSuggestions()],
  plans: PLANS,
  goals: GOALS,
  alerts: ALERTS,
  watchlists: [keys.watchlists(), keys.watchlistRows()],
  accounts: [...LISTS, keys.scope(), ...REPORTS, ...GOALS],
  // Naming a region is a portfolio change, and it is what real returns are measured against.
  portfolio: [keys.portfolio(), keys.status(), keys.scope(), keys.inflationStatus(), ...LISTS, ...REPORTS],
  securities: [
    ...LISTS,
    ...PLANS,
    // Deleting an instrument takes it off every list.
    keys.watchlists(),
    keys.attributeDefs(),
    keys.taxonomyGrouping(),
    keys.coverage(),
    keys.quotes(),
    keys.listings(),
    keys.corporateActions(),
    // A recorded split marks the reported one; a switched listing drops what it reported.
    ...ALERTS,
    ...REPORTS,
  ],
  // A fresh quote changes both the last close on the row and every valuation.
  // The same refresh that fetches quotes fetches the month's price index.
  quotes: [
    keys.securities(),
    keys.quotes(),
    keys.coverage(),
    keys.refreshStatus(),
    keys.inflationStatus(),
    ...ALERTS,
    ...REPORTS,
    // A goal's progress is a valuation, so a fresh price moves it.
    ...GOALS,
  ],
  taxonomies: TREES,
  targets: [keys.targets(), keys.rebalance(), keys.allocation()],
  // A scope narrows the point of view, so every computed number moves; stored lists do not.
  scope: [keys.scope(), ...REPORTS],
};

/**
 * Invalidates the listed keys. Pass a group: `invalidate(...affects.accounts)`.
 * Keys are prefixes, so one entry covers every date or filter under it.
 */
export function useInvalidate() {
  const client = useQueryClient();
  // Stable across renders: callers put it in an effect's dependency list.
  return useCallback(
    (...invalidated: QueryKey[]) => {
      for (const queryKey of invalidated) client.invalidateQueries({ queryKey });
    },
    [client],
  );
}
