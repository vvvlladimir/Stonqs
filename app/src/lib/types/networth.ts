/** Things owned and owed beside the portfolio, and the net-worth reading they make (ADR-0092). */

import type { ImportProblem } from "./imports";
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

/** Where a debt is going on the owner's own assumptions; it never restates the balance. */
export interface DebtPayoff {
  /** `null` when the payment does not cover the interest: there is no end, which is not "now". */
  months_left: number | null;
  payoff_on: DateString | null;
  interest_ahead: string | null;
  last_payment: string | null;
  /** The end the contract names, for comparison with `payoff_on`. */
  ends_on: DateString | null;
  /** From the first figure ever written to today's; negative when the debt grew. */
  paid_share: string | null;
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
  days_old: number;
  /** Past the app's fuse (half a year), so the row says the figure is old. */
  stale: boolean;
  /** What the last revaluation changed, both figures at the reading date's rate. */
  change_base: string | null;
  changed_since: DateString | null;
  secured_by: string | null;
  /** For a thing owned: what is owed against it, and what is left after that. */
  secured_debt_base: string | null;
  equity_base: string | null;
  payoff: DebtPayoff | null;
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
  /** Everything owed over everything owned, investments included. */
  debt_to_assets: string | null;
  /** How many figures are older than half a year. */
  stale_count: number;
  holdings: AssetHolding[];
  /** Ids of assets with no valuation on or before the date — absent, not zero. */
  not_valued_yet: string[];
}

/** What selling the portfolio today would cost in tax, on a rate the owner states (ADR-0093). */
export interface AfterTax {
  /** The rate as a fraction, echoed back so the screen can say what it assumed. */
  rate: string;
  /** The portfolio's unrealized gain, never below zero: a loss is not a refund. */
  taxable_gain_base: string;
  tax_base: string;
  net_after_tax_base: string;
  /** What this reading says nothing about: everything owned with no purchase price. */
  outside_base: string;
}

export interface NetWorthData {
  reading: NetWorth;
  /** Absent until a rate is stated; never a zero tax nobody asked for. */
  after_tax: AfterTax | null;
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

/** One row of a valuations CSV, matched to a thing by name or not matched at all. */
export interface ValueRow {
  row: number;
  label: string;
  asset_id: string | null;
  date: DateString;
  amount: string;
  /** A figure for this thing on this day is already stored; the row replaces it. */
  replaces: boolean;
}

export interface ValuesCsvConfig {
  name: string | null;
  date: string | null;
  amount: string | null;
}

/** What a valuations file would write. Nothing is written until the commit. */
export interface ValuesPreview {
  config: ValuesCsvConfig;
  rows: ValueRow[];
  /** Names the portfolio has no thing for; nothing is created from a file. */
  unmatched: string[];
  problems: ImportProblem[];
}
