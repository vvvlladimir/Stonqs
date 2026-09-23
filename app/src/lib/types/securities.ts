/** Instruments: what identifies one, what a listing is, and what happened to it. */

import type { ImportProblem } from "./imports";
import type { DateString, MoneyString } from "./primitives";
export type SecurityKind = "STOCK" | "ETF" | "BOND" | "FUND" | "CRYPTO" | "OTHER";

export interface Security {
  id: string;
  symbol: string;
  isin: string | null;
  name: string;
  currency: string;
  kind: SecurityKind;
  data_source: string | null;
  data_symbol: string | null;
  /** Null means the step is inferred from the security kind. */
  quantity_step: MoneyString | null;
  /** Selected venue code under ISO 10383, or null when unset. */
  mic: string | null;
  /** German securities number, printed beside the ISIN by German brokers. */
  wkn: string | null;
  /** The user's own note; no provider ever writes it. */
  note: string | null;
}

/** How an attribute's value is read; it is stored as text in every case. */
export type AttributeKind = "TEXT" | "NUMBER" | "DATE";

/** An attribute the user invented: TER, country of risk, replication method. */
export interface SecurityAttributeDef {
  id: string;
  name: string;
  kind: AttributeKind;
  /** Shown after the value ("%", "bp"); never parsed. */
  unit: string | null;
  position: number;
}

/** What an attribute CSV would do, before it does it. */
export interface AttributeCsvConfig {
  symbol: string | null;
  isin: string | null;
  name: string | null;
  /** Every other column of the file, by header. */
  attributes: string[];
}

export interface PreviewAttribute {
  name: string;
  kind: AttributeKind;
  /** The attribute this column already is; `null` means the commit would create it. */
  attribute_id: string | null;
  values: number;
}

export interface AttributeImportRow {
  row: number;
  label: string;
  symbol: string;
  isin: string;
  security_id: string | null;
  /** `isin`, `symbol`, `symbol_base` or `name` — a code, not a label. */
  matched_by: string | null;
  /** Attribute name to the value it would be given; a blank cell is absent. */
  values: Record<string, string>;
}

export interface AttributePreview {
  config: AttributeCsvConfig;
  attributes: PreviewAttribute[];
  rows: AttributeImportRow[];
  problems: ImportProblem[];
}

export interface AttributeImportResult {
  attributes_created: number;
  instruments: number;
  values: number;
}

export interface AttributeDefInput {
  id: string | null;
  name: string;
  kind: AttributeKind;
  unit: string | null;
  position: number;
}

/** Attribute values of one instrument, keyed by attribute id. */
export type AttributeValues = Record<string, string>;

export interface SecurityRow extends Security {
  transaction_count: number;
  /** Symbols at sources other than `data_source`, `source -> symbol`. */
  other_symbols: Record<string, string>;
  /** Effective step: explicit, trade-observed, then kind default. */
  effective_quantity_step: MoneyString;
  /** Step observed in trades, or null when no quantities were recorded. */
  quantity_step_observed: MoneyString | null;
  /** True when the symbol is an ISIN that needs provider resolution. */
  needs_lookup: boolean;
  /** Currency of the latest quote, or null when no quotes exist. */
  quote_currency: string | null;
  /** Venue name resolved from MIC, or null when unset. */
  venue: string | null;
  /** Quote count and coverage period stored locally. */
  quote_count: number;
  coverage_from: DateString | null;
  coverage_to: DateString | null;
  /** Latest close in `quote_currency`, or null without quotes. */
  last_close: MoneyString | null;
  /** The stored series is far shorter than the instrument has been held: usually a wrong venue. */
  sparse_history: boolean;
  attributes: AttributeValues;
}

/** One venue where an instrument trades. */
export interface Listing {
  isin: string;
  /** Venue code under ISO 10383. */
  mic: string;
  ticker: string;
  exchange: string | null;
  name: string | null;
  /** Provider symbol, or null when the provider cannot represent the listing. */
  symbol: string | null;
  /** Null means the listing has not been checked. */
  currency: string | null;
  has_history: boolean | null;
  last_close: MoneyString | null;
  source: string;
}

export interface ListingChoice {
  security_id: string;
  symbol: string;
  currency: string;
  /** Selected venue; it cannot be recovered from the symbol alone. */
  mic: string | null;
}

export interface SecurityInput {
  id: string | null;
  symbol: string;
  name: string;
  currency: string;
  kind: SecurityKind;
  isin: string | null;
  /** Absent keeps the venue already on the instrument; only the venue picker clears one. */
  mic?: string | null;
  data_source: string | null;
  data_symbol: string | null;
  quantity_step: string | null;
  wkn: string | null;
  note: string | null;
  /** Absent means the caller does not edit attributes; an empty map clears every one. */
  attributes?: AttributeValues;
  /** Symbols at other sources (`source -> symbol`), asked when the own source fails. Absent
   * means not edited; a blank value forgets that source. */
  other_symbols?: Record<string, string>;
}

export interface SecurityRef {
  id: string;
  symbol: string;
  name: string;
  currency: string;
}

export interface DataCoverage {
  security_id: string;
  symbol: string;
  data_source: string | null;
  /** Date of the latest quote, or null when no quotes exist. */
  last_quote: DateString | null;
  first_quote: DateString | null;
  quote_count: number;
  /** Calendar days since the latest quote. */
  stale_days: number | null;
  /** Currency of the latest quote. */
  quote_currency: string | null;
  mic: string | null;
  venue: string | null;
}

/** Payment schedule inferred from the gaps between an instrument's own payments. */
export type DividendFrequency = "MONTHLY" | "QUARTERLY" | "SEMI_ANNUAL" | "ANNUAL" | "IRREGULAR" | "UNKNOWN";

/** Only splits for now: the one quantity-changing event that moves no money. */
export type CorporateActionKind = "SPLIT";

/**
 * A split as stored: `ratio_from`/`ratio_to` read "one old share becomes two" for a 2:1,
 * and `10 -> 1` for a 1:10 reverse split. Quotes arrive already adjusted, so this only
 * moves lots.
 */
export interface CorporateAction {
  id: string;
  security_id: string;
  /** Ex-date: the new quantity applies from this day on. */
  date: DateString;
  kind: CorporateActionKind;
  ratio_from: MoneyString;
  ratio_to: MoneyString;
  note: string | null;
}

export interface CorporateActionRow extends CorporateAction {
  symbol: string;
  name: string;
}

export interface CorporateActionInput {
  id?: string | null;
  security_id: string;
  date: DateString;
  ratio_from: MoneyString;
  ratio_to: MoneyString;
  note?: string | null;
}

/** The user's note, or a dividend or split the quote provider reported. */
export type SecurityEventKind = "NOTE" | "DIVIDEND" | "SPLIT";

export interface SecurityEvent {
  id: string;
  security_id: string;
  date: DateString;
  kind: SecurityEventKind;
  /** Dividend per share, in `currency`. */
  amount: MoneyString | null;
  currency: string | null;
  ratio_from: MoneyString | null;
  ratio_to: MoneyString | null;
  note: string | null;
  /** Provider id; null is the user's own event. */
  source: string | null;
}

export interface SecurityEventRow extends SecurityEvent {
  symbol: string;
  name: string;
  /** A reported split already recorded as a split of the instrument. */
  recorded: boolean;
}

export interface SecurityEventInput {
  id?: string | null;
  security_id: string;
  date: DateString;
  note: string;
}
