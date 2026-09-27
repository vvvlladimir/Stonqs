/** Instruments and what hangs off them: attributes, splits, alerts, events, listings, watchlists. */

import type {
  AlertInput,
  AlertRow,
  CrossingRow,
  SecurityAlert,
  SecurityEvent,
  SecurityEventInput,
  SecurityEventRow,
  AttributeDefInput,
  SecurityAttributeDef,
  CorporateAction,
  CorporateActionInput,
  CorporateActionRow,
  Quote,
  SecurityMatch,
  DateString,
  Security,
  SecurityInput,
  Listing,
  ListingChoice,
  SecurityRow,
  Watchlist,
  WatchlistInput,
  WatchRow,
} from "../types";
import { call } from "./core";

export const securitiesApi = {
  securitiesList: () => call<SecurityRow[]>("securities_list"),
  securitySave: (input: SecurityInput) => call<Security>("security_save", { input }),
  securityDelete: (id: string) => call<void>("security_delete", { id }),
  /** The attributes the user defined; their values travel on the security row. */
  attributeDefsList: () => call<SecurityAttributeDef[]>("attribute_defs_list"),
  attributeDefSave: (input: AttributeDefInput) => call<SecurityAttributeDef>("attribute_def_save", { input }),
  attributeDefDelete: (id: string) => call<void>("attribute_def_delete", { id }),
  /** Splits of one instrument, or of every instrument when the id is omitted. */
  corporateActionsList: (security_id?: string) =>
    call<CorporateActionRow[]>("corporate_actions_list", { securityId: security_id ?? null }),
  corporateActionSave: (input: CorporateActionInput) =>
    call<CorporateAction>("corporate_action_save", { input }),
  corporateActionDelete: (id: string) => call<void>("corporate_action_delete", { id }),
  /** Limit prices and date rules with what each says today; every instrument without an id. */
  alertsList: (security_id?: string) => call<AlertRow[]>("alerts_list", { securityId: security_id ?? null }),
  alertSave: (input: AlertInput) => call<SecurityAlert>("alert_save", { input }),
  alertDelete: (id: string) => call<void>("alert_delete", { id }),
  /** The crossing log across every rule, newest first. */
  alertCrossingsList: (limit?: number) =>
    call<CrossingRow[]>("alert_crossings_list", { limit: limit ?? null }),
  /** How many crossings were not looked at; the navigation shows a dot while any are left. */
  alertsUnseen: () => call<number>("alerts_unseen"),
  alertsMarkSeen: () => call<number>("alerts_mark_seen"),
  /** Checks every rule, then hands over the crossings not announced yet, marked as announced. */
  alertsTakeNotifications: () => call<CrossingRow[]>("alerts_take_notifications"),
  /** Notes, dividends and splits, newest first; every instrument without an id. */
  securityEventsList: (security_id?: string) =>
    call<SecurityEventRow[]>("security_events_list", { securityId: security_id ?? null }),
  securityEventSave: (input: SecurityEventInput) => call<SecurityEvent>("security_event_save", { input }),
  securityEventDelete: (id: string) => call<void>("security_event_delete", { id }),
  /** Named lists of instruments; not scoped, like the alerts on them. */
  watchlistsList: () => call<Watchlist[]>("watchlists_list"),
  watchlistSave: (input: WatchlistInput) => call<Watchlist>("watchlist_save", { input }),
  watchlistDelete: (id: string) => call<void>("watchlist_delete", { id }),
  /** Each instrument's own price facts over the period, in the list's order. */
  watchlistRows: (id: string, from: DateString, to: DateString) =>
    call<WatchRow[]>("watchlist_rows", { id, from, to }),

  /** Identifies an existing security and updates its market metadata. */
  securityIdentify: (id: string) => call<Security>("security_identify", { id }),
  /** Cached quotes for a period; this command does not use the network. */
  quotesRange: (security_id: string, from: DateString, to: DateString) =>
    call<Quote[]>("quotes_range", { securityId: security_id, from, to }),
  /** Security venues; `refresh` forces a new directory lookup. */
  securityListings: (id: string, refresh: boolean) => call<Listing[]>("security_listings", { id, refresh }),
  /** Changes the listing and clears quotes for the previous symbol. */
  securitySetListing: (choice: ListingChoice) => call<Security>("security_set_listing", { choice }),

  securitySearch: (query: string) => call<SecurityMatch[]>("security_search", { query }),
  /** The exact listing's profile, currency included; never another listing of the same name. */
  securityProfile: (source: string, symbol: string) =>
    call<SecurityMatch | null>("security_profile", { source, symbol }),
  securityResolve: (query: string, currency: string | null = null) =>
    call<SecurityMatch | null>("security_resolve", { query, currency }),
};
