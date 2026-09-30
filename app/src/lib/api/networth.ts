/** Net worth: the portfolio plus what else is owned, minus what is owed (ADR-0092). */

import type {
  Asset,
  AssetInput,
  AssetValue,
  AssetValueInput,
  DateString,
  NetWorthData,
  NetWorthSeries,
} from "../types";
import { call } from "./core";

export const netWorthApi = {
  /** The reading at `date`, with every asset. Not scoped: an asset is not an account. */
  netWorth: (date: DateString) => call<NetWorthData>("net_worth", { date }),
  /** The line between two days; its portfolio side is the performance screen's own series. */
  netWorthSeries: (from: DateString, to: DateString) =>
    call<NetWorthSeries>("net_worth_series", { from, to }),
  /** Every figure ever written for one thing, oldest first. */
  assetValues: (asset_id: string) => call<AssetValue[]>("asset_values", { assetId: asset_id }),
  assetSave: (input: AssetInput) => call<Asset>("asset_save", { input }),
  assetDelete: (id: string) => call<void>("asset_delete", { id }),
  /** One figure per day: the same day twice replaces that day's answer. */
  assetValueSave: (input: AssetValueInput) => call<void>("asset_value_save", { input }),
  assetValueDelete: (asset_id: string, date: DateString) =>
    call<void>("asset_value_delete", { assetId: asset_id, date }),
};
