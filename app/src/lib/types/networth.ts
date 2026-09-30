/** Things owned and owed beside the portfolio, and the net-worth reading they make (ADR-0092). */

import type { DateString } from "./primitives";

/** Which way an amount points. Both sides are stored positive; the kind decides the direction. */
export type AssetSide = "OWNED" | "OWED";

export type AssetKind =
  | "PROPERTY"
  | "VEHICLE"
  | "COLLECTIBLE"
  | "CASH"
  | "PRIVATE"
  | "RECEIVABLE"
  | "OTHER"
  | "MORTGAGE"
  | "LOAN"
  | "CREDIT_CARD"
  | "CREDIT_LINE"
  | "TAX_DUE";

/** What a debt costs and when it ends. Looks forward only: it never restates the balance. */
export interface Amortization {
  /** A fraction a year: `"0.0345"` is 3.45%. */
  rate: string;
  monthly_payment: string;
  ends_on: DateString | null;
}

export interface Asset {
  id: string;
  name: string;
  kind: AssetKind;
  currency: string;
  /** The debt's asset, or the asset's debt. A relationship, not a figure. */
  secured_by: string | null;
  schedule: Amortization | null;
  note: string | null;
  closed_on: DateString | null;
}

/** What somebody said a thing was worth on one day, in the asset's own currency. */
export interface AssetValue {
  asset_id: string;
  date: DateString;
  amount: string;
  note: string | null;
}

export interface AssetHolding {
  asset_id: string;
  name: string;
  kind: AssetKind;
  side: AssetSide;
  currency: string;
  amount: string;
  amount_base: string;
  /** The day the figure is from, which is not the reading date. */
  valued_on: DateString;
  secured_by: string | null;
}

export interface NetWorth {
  date: DateString;
  base_currency: string;
  investments_base: string;
  owned_base: string;
  owed_base: string;
  net_base: string;
  /** `null` when net worth is zero or below: there is no share to state. */
  invested_share: string | null;
  holdings: AssetHolding[];
  /** Ids of assets with no valuation on or before the date — absent, not zero. */
  not_valued_yet: string[];
}

export interface NetWorthData {
  reading: NetWorth;
  /** Every asset, valued or not, so the ones waiting for a first figure still get a row. */
  assets: Asset[];
}

export interface NetWorthPoint {
  date: DateString;
  investments_base: string;
  owned_base: string;
  owed_base: string;
  net_base: string;
}

export interface NetWorthSeries {
  base_currency: string;
  points: NetWorthPoint[];
}

export interface AssetInput {
  id: string | null;
  name: string;
  kind: AssetKind;
  currency: string;
  secured_by: string | null;
  note: string | null;
  closed_on: DateString | null;
  /** Percent a year, as typed: `"3.45"`. Only a debt has one. */
  rate: string | null;
  monthly_payment: string | null;
  ends_on: DateString | null;
  /** A first valuation, accepted only while the asset is being created. */
  amount: string | null;
  valued_on: DateString | null;
}

export interface AssetValueInput {
  asset_id: string;
  date: DateString;
  amount: string;
  note: string | null;
}
