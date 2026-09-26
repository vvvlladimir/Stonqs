/** Quotes, and the background job that fetches them. */

import type { ImportProblem } from "./imports";
import type { DateString, MoneyString } from "./primitives";
export interface Quote {
  security_id: string;
  date: DateString;
  close: MoneyString;
  currency: string;
  source: string;
}

export interface PriceImport {
  quotes: Quote[];
  problems: ImportProblem[];
  unknown_symbols: string[];
}

// Background jobs

/** Why a refresh step failed; the UI writes the headline from `code`. */
export type FailureCode =
  "database" | "securities" | "currencies" | "quote" | "rate" | "unidentified" | "internal";

/** What went wrong underneath a failure; the UI turns it into what the user should fix. */
export type FailureCause = "unreachable" | "rejected" | "unauthorized" | "no_data" | "storage" | "other";

export interface JobFailure {
  code: FailureCode;
  cause: FailureCause;
  /** A symbol, a currency pair, or a comma-separated list. */
  subject: string;
  /** English detail from the host, shown under the localized headline. */
  detail: string;
  /** The source that was asked and failed; null when none was reached. */
  source: string | null;
}

/** Refresh progress event shared by the application. */
export type Progress =
  | { event: "started"; total: number }
  | { event: "item"; label: string; done: number; total: number; fetched: number }
  | { event: "finished"; fetched: number; failed: JobFailure[]; cancelled: boolean };

export interface RefreshStatus {
  running: boolean;
  label: string | null;
  done: number;
  total: number;
  fetched: number;
  /** RFC 3339 timestamp of the last completed refresh. */
  last_finished: string | null;
  failures: JobFailure[];
  /** Whether the last refresh was cancelled by the user. */
  cancelled: boolean;
}

export type RefreshMode = "catch_up" | "full";

/** One market-data source this build ships, with what the user decided about it. */
export interface MarketSourceRow {
  id: string;
  /** The provider's own site: where this source's requests go, and whose terms apply. */
  site: string;
  capabilities: ("quotes" | "search" | "listings" | "fx_rates" | "price_index")[];
  key: "none" | "optional" | "required";
  has_key: boolean;
  on_by_default: boolean;
  /** Switched on, by the user or by default — what was picked, whether or not it is asked yet. */
  wanted: boolean;
  /** Actually asked: wanted, not missing a required key, and the sources confirmed. */
  active: boolean;
}

export type CustomFormat =
  | { kind: "json"; date_path: string; close_path: string }
  | { kind: "csv"; date_column: string; close_column: string };

/** A quote source the user describes: a URL template and where the dates and closes are. */
export interface CustomSource {
  /** `custom:<slug>`. */
  id: string;
  label: string;
  /** `fx` takes `{BASE}` and `{QUOTE}` in the address; each close is quote units per one base. */
  role: "quotes" | "fx";
  url: string;
  headers: { name: string; value: string }[];
  format: CustomFormat;
  date_format: string | null;
  factor: MoneyString | null;
  currency: string | null;
}

export interface CustomTestRow {
  date: DateString;
  close: MoneyString;
  currency: string;
}
