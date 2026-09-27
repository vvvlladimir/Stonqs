/** Instruments, alerts, events, watchlists, listings and quote windows. */

import { useQuery } from "@tanstack/react-query";
import { api, today } from "../api";
import type { DateString, PeriodRange } from "../types";
import { keys } from "./keys";

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

/** The crossing log across every rule, newest first. */
export function useAlertCrossings(limit: number) {
  return useQuery({ queryKey: keys.alertCrossings(limit), queryFn: () => api.alertCrossingsList(limit) });
}

/** Crossings not looked at yet: the dot beside Alerts in the navigation. */
export function useAlertsUnseen() {
  return useQuery({ queryKey: keys.alertsUnseen(), queryFn: api.alertsUnseen });
}

/** Rules with their status: of one instrument, or of every instrument when the id is absent. */
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
