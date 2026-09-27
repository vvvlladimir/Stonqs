/** What a write invalidates, grouped by the `scope` the host puts on `data:changed`. */

import { useCallback } from "react";
import { useQueryClient, type QueryKey } from "@tanstack/react-query";
import type { DataChangeKind } from "../types";
import { keys } from "./keys";

/** Every key computed from transactions and prices — a report, not a stored list. */
const REPORTS: QueryKey[] = [
  keys.ledgerGaps(),
  keys.positions(),
  keys.pluginReads(),
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
