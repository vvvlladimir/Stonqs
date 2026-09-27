/** Quote refresh and the market-data sources. */

import type {
  DataCoverage,
  RefreshMode,
  RefreshStatus,
  MarketSourceRow,
  CustomSource,
  CustomTestRow,
} from "../types";
import { call } from "./core";

export const marketApi = {
  dataCoverage: () => call<DataCoverage[]>("data_coverage"),
  /** Returns false when a refresh is already running. */
  marketRefresh: (mode: RefreshMode) => call<boolean>("market_refresh", { mode }),
  /** Requests cancellation between securities. */
  marketRefreshCancel: () => call<void>("market_refresh_cancel"),
  refreshStatus: () => call<RefreshStatus>("refresh_status"),
  /** Registered quote providers for the security screen. */
  quoteProviders: () => call<string[]>("quote_providers"),
  marketSources: () => call<MarketSourceRow[]>("market_sources_list"),
  marketSourceSwitch: (source: string, on: boolean) => call<void>("market_source_switch", { source, on }),
  /** Seals the answer to "where may data come from"; until it is called nothing is fetched. */
  marketSourcesConfirm: () => call<void>("market_sources_confirm"),
  /** Gives every instrument with no price source the one named; returns how many took it. */
  securitiesAdoptSource: (source: string) => call<number>("securities_adopt_source", { source }),
  marketKeySave: (source: string, key: string) => call<void>("market_key_save", { source, key }),
  marketKeyDelete: (source: string) => call<void>("market_key_delete", { source }),
  marketCustom: () => call<CustomSource[]>("market_custom_list"),
  marketCustomSave: (source: CustomSource) => call<void>("market_custom_save", { source }),
  marketCustomDelete: (id: string) => call<void>("market_custom_delete", { id }),
  marketCustomTest: (source: CustomSource, symbol: string, currency: string) =>
    call<CustomTestRow[]>("market_custom_test", { source, symbol, currency }),
};
