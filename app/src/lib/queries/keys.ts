/** Every query key, spelled once. */

import { type QueryKey } from "@tanstack/react-query";
import { type Source } from "../api";
import type {
  DateString,
  PaymentPeriod,
  SheetPeriod,
  TradeGrouping,
  TransactionFilter,
  TransactionKind,
  WidgetRead,
} from "../types";

/** Trailing `undefined`s are dropped so a shorter key is a prefix; `null` stays. */
function key(...parts: unknown[]): QueryKey {
  while (parts.length > 0 && parts[parts.length - 1] === undefined) parts.pop();
  return parts;
}

export const keys = {
  status: () => key("status"),
  settings: () => key("settings"),
  profiles: () => key("profiles"),
  plugins: () => key("plugins"),
  pluginTheme: (key_?: string) => key("plugin-theme", key_),
  pluginState: (plugin?: string) => key("plugin-state", plugin),
  pluginReads: (
    reads?: readonly WidgetRead[],
    date?: DateString,
    from?: DateString,
    to?: DateString,
    source?: Source,
  ) => key("plugin-reads", reads?.join(","), date, from, to, source ?? undefined),
  portfolio: () => key("portfolio"),
  ledgerGaps: () => key("ledger-gaps"),
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
  netWorth: (date?: DateString, taxRate?: string) => key("net-worth", date, taxRate),
  netWorthSeries: (from?: DateString, to?: DateString) => key("net-worth-series", from, to),
  assetValues: (assetId?: string) => key("asset-values", assetId),
  limits: (date?: DateString) => key("limits", date),
  trades: (from?: DateString, to?: DateString, by?: TradeGrouping, source?: Source) =>
    key("trades", from, to, by, source ?? undefined),
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
