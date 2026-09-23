/** CSV import, from parse settings to the committed result. */

import type { DateString, MoneyString } from "./primitives";
import type { SecurityKind } from "./securities";
import type { TransactionKind } from "./transactions";
export interface ParseConfig {
  delimiter: string | null;
  skip_top_rows: number;
  skip_bottom_rows: number;
  has_header: boolean | null;
  date_format: string | null;
  decimal_separator: string | null;
}

export type ImportField =
  | "DATE"
  | "KIND"
  | "SYMBOL"
  | "ISIN"
  | "NAME"
  | "QUANTITY"
  | "PRICE"
  | "AMOUNT"
  | "FEE"
  | "TAX"
  | "CURRENCY"
  | "FEE_CURRENCY"
  | "TAX_CURRENCY"
  | "FX_RATE"
  | "ACCOUNT"
  | "LINK_ID"
  | "NOTE";

export interface ImportMapping {
  account_id: string | null;
  /** Import field mapped to a file column. */
  columns: Partial<Record<ImportField, string>>;
  kind_aliases: Record<string, TransactionKind>;
  /** Normalized operation values left out of the import on purpose. */
  ignored_kinds: string[];
  symbol_aliases: Record<string, string>;
  account_aliases: Record<string, string>;
  default_currency: string | null;
  /** Whether the amount sign encodes transaction direction. */
  amount_sign: AmountSign | null;
  /** Whether the amount column already has the row's charges in it. */
  amount_basis: AmountBasis | null;
  /** What a row becomes when its wording is not the whole answer. */
  rules: ImportRule[];
  /** Normalized file symbol mapped to a security to create. */
  new_securities: Record<string, SecurityDraft>;
}

/** Instrument returned by a market-data search. */
export interface SecurityMatch {
  source: string;
  symbol: string;
  name: string;
  exchange: string | null;
  /** Venue the source quotes this symbol on, or null when it names none. */
  mic: string | null;
  kind: SecurityKind;
  /** Null for search results; currency arrives with the full profile. */
  currency: string | null;
  isin: string | null;
  /** Whether the provider has price history for this symbol. */
  has_history: boolean | null;
}

/** Security draft created by import after resolution. */
export interface SecurityDraft {
  symbol: string;
  name: string;
  currency: string;
  kind: SecurityKind;
  isin: string | null;
  data_source: string | null;
  data_symbol: string | null;
  exchange: string | null;
  /** ISO 10383 code behind `exchange`, when the source named a supported venue. */
  mic: string | null;
}

export interface RowOverride {
  number: number;
  field: ImportField;
  value: string;
}

export type RowStatus =
  | "READY"
  | "DUPLICATE"
  /** The broker restated a row we already hold; importing replaces it. */
  | "UPDATED"
  | "UNKNOWN_SECURITY"
  /** A stored operation of the same day, account, instrument and quantity is worth something
   *  else: the likeliest reading is that it is this row, corrected by hand since. */
  | "SIMILAR"
  | "IGNORED"
  | "INVALID";

/** Whether a statement amount sign determines transaction direction. */
export type AmountSign = "SIGNED" | "UNSIGNED";
export type AmountBasis = "GROSS" | "NET";

/** Import severity: errors block writing; warnings remain reviewable. */
export type Severity = "ERROR" | "WARNING";

/** Stable problem code used to group repeated import messages. */
export type ProblemCode =
  | "ENCODING"
  | "MALFORMED_ROW"
  | "MISSING_COLUMN"
  | "NOT_A_NUMBER"
  | "BAD_DATE"
  | "MISSING_VALUE"
  | "UNKNOWN_KIND"
  | "UNKNOWN_ACCOUNT"
  | "WRONG_ACCOUNT_KIND"
  | "TRANSFER_WITH_SECURITY"
  | "INVALID_TRANSACTION"
  | "DUPLICATE_IN_STORE"
  | "DUPLICATE_IN_FILE"
  | "RESTATED_IN_STORE"
  | "SECURITY_WITHOUT_SOURCE"
  | "DIRECTION_FROM_SIGN"
  | "DIRECTION_CONFLICT"
  | "AMOUNT_SIGN_AMBIGUOUS"
  | "AMOUNT_BASIS_AMBIGUOUS"
  | "AMOUNT_VS_QUANTITY_PRICE"
  | "FEE_EXCEEDS_AMOUNT"
  | "FX_RATE_ON_BASE_CURRENCY"
  | "SINGLE_KIND_VALUE"
  | "FUTURE_DATE"
  | "IMPLAUSIBLE_DATE_SPAN"
  | "ZERO_AMOUNT"
  | "SUSPICIOUS_CURRENCY"
  | "DELIVERY_WITHOUT_COST"
  | "ACCOUNT_CURRENCY_MISMATCH"
  | "TICKER_ISIN_CONFLICT"
  | "SIMILAR_IN_STORE"
  | "POSSIBLE_SPLIT";

export interface ImportProblem {
  row: number | null;
  column: string | null;
  severity: Severity;
  code: ProblemCode;
  /** English fallback wording from the core. */
  message: string;
  /** Values for the UI's own sentence; absent when the code needs none. */
  params?: Record<string, string>;
}

export interface TransactionDraft {
  account_id: string;
  kind: TransactionKind;
  date: DateString;
  symbol: string | null;
  isin: string | null;
  security_name: string | null;
  security_id: string | null;
  quantity: MoneyString;
  price: MoneyString;
  amount: MoneyString;
  fees: MoneyString;
  taxes: MoneyString;
  currency: string;
  /** Currency the charge was billed in; null when it is the operation's own. */
  fee_currency: string | null;
  tax_currency: string | null;
  fx_rate_to_base: MoneyString | null;
  link_id: string | null;
  /** The broker's own identifier for the row, when the file carried one. */
  external_id: string | null;
  /** Stored operation this row restates, decided by the preview. */
  replaces: string | null;
  note: string | null;
}

/** What a field has to look like for a rule to apply. */
export type RuleTest =
  | { equals: string }
  | { contains: string }
  | { sign: "POSITIVE" | "NEGATIVE" | "ZERO" }
  | "present"
  | "empty";

export interface RuleCondition {
  field: ImportField;
}

/** One operation a rule produces; `set` replaces the row's own values by field. */
export interface RuleEmit {
  kind: TransactionKind;
  set?: Partial<Record<ImportField, string>>;
}

/** A match and the operations it produces — an empty `emit` leaves the row out. */
export interface ImportRule {
  when: Array<RuleCondition & RuleTest>;
  emit: RuleEmit[];
  link?: boolean;
}

export interface ImportRow {
  number: number;
  /** Which operation of that file row this is, 1-based; several share one number. */
  part: number;
  raw: Record<string, string>;
  draft: TransactionDraft | null;
  status: RowStatus;
  problems: ImportProblem[];
}

export interface ImportSummary {
  total: number;
  ready: number;
  duplicates: number;
  /** Rows a stored operation resembles closely enough to be the same one, edited since. */
  similar: number;
  /** Rows that replace an operation already stored, matched by the broker's identifier. */
  updated: number;
  unknown_securities: number;
  /** Rows whose operation value the user chose to skip. */
  ignored: number;
  invalid: number;
  /** Warning rows that can be imported after review. */
  warnings: number;
}

/** One distinct transaction-kind value and its current mapping. */
export interface KindMapping {
  value: string;
  count: number;
  kind: TransactionKind | null;
  /** Left out of the import on purpose rather than left unmapped. */
  ignored: boolean;
}

export interface AccountMapping {
  value: string;
  count: number;
  account_id: string | null;
}

export interface SymbolMapping {
  /** Raw value from the file. */
  value: string;
  /** Normalized value used for resolution. */
  resolved: string;
  isin: string | null;
  /** File name used as a draft for a new security. */
  file_name: string | null;
  count: number;
  security_id: string | null;
  name: string | null;
  currency: string | null;
  /** Security to create for this symbol. */
  planned: SecurityDraft | null;
  /** Whether at least one row requires this security. */
  required: boolean;
}

export interface ImportPreview {
  config: ParseConfig;
  mapping: ImportMapping;
  rows: ImportRow[];
  problems: ImportProblem[];
  kinds: KindMapping[];
  symbols: SymbolMapping[];
  accounts: AccountMapping[];
  /** Whether the amount sign was chosen or inferred. */
  amount_sign: AmountSign;
  amount_basis: AmountBasis;
  summary: ImportSummary;
}

/** Where a saved layout came from: the user made it, or the app ships it. */
export type TemplateSource = "USER" | "BUILTIN";

/** Saved import mapping for one broker. */
export interface ImportTemplate {
  name: string;
  config: ParseConfig;
  mapping: ImportMapping;
  source: TemplateSource;
}

/** Preview plus file headers for column mapping. */
export interface ImportPreviewData extends ImportPreview {
  headers: string[];
  /** Layout the file was recognised as when it was loaded; absent when nothing fitted. */
  applied_template?: string;
}

export interface ImportOptions {
  create_missing_securities: boolean;
  new_security_kind: SecurityKind;
  /** Quote provider for created securities, or null for manual prices. */
  /** Absent means the host's default source; `null` means manual prices. */
  new_security_source?: string | null;
  import_duplicates: boolean;
  /** Write rows a stored operation only resembles. Off by default. */
  import_similar: boolean;
}

export interface ImportResult {
  imported: number;
  /** Rows that replaced a stored operation rather than adding one. */
  updated: number;
  skipped: number;
  /** Rows skipped because a stored operation resembles them. */
  similar: number;
  created_securities: string[];
  problems: ImportProblem[];
}
