/** The payments grid: one line per payer, one amount per bucket. */

import type { DateString, MoneyString } from "./primitives";
import type { DividendFrequency } from "./securities";
/** Width of one column of the payments grid. */
export type PaymentPeriod = "MONTH" | "QUARTER" | "YEAR";

/** The rows of the grid. Income kinds are named one by one; everything else is one line. */
export type PaymentLine =
  | "DIVIDENDS"
  | "INTEREST"
  | "INTEREST_CHARGE"
  | "OTHER_INCOME"
  | "FEES"
  | "TAXES"
  | "SAVINGS"
  | "CLOSED_TRADES";

/** One column: an interval, not a label — the month's name is written in the frontend. */
export interface PaymentBucket {
  from: DateString;
  to: DateString;
  year: number;
  /** 1..12 for a month, 1..4 for a quarter, always 1 for a year. */
  index: number;
}

export interface PaymentRow {
  line: PaymentLine;
  /** One amount per bucket, in the order of `buckets`. */
  amounts: MoneyString[];
  total: MoneyString;
}

export interface PayerRow {
  /** Null is the account itself: interest owes nothing to an instrument. */
  security_id: string | null;
  amounts: MoneyString[];
  total: MoneyString;
  symbol: string;
  name: string;
}

export interface PaymentsData {
  from: DateString;
  to: DateString;
  base_currency: string;
  period: PaymentPeriod;
  buckets: PaymentBucket[];
  lines: PaymentRow[];
  payers: PayerRow[];
  /** Income only, per bucket, and its running sum beside it. */
  earnings: MoneyString[];
  cumulative: MoneyString[];
  earnings_total: MoneyString;
}

/** One dividend the open positions should pay, projected from the instrument's reported past. */
export interface ExpectedDividendRow {
  security_id: string;
  symbol: string;
  name: string;
  ex_date: DateString;
  /** Null when no payment was ever received to measure the lag from ex-date to cash. */
  pay_date: DateString | null;
  frequency: DividendFrequency;
  /** The source already reported this payment; otherwise date and amount are carried forward. */
  reported: boolean;
  per_share: MoneyString;
  currency: string;
  quantity: MoneyString;
  gross_base: MoneyString;
  /** Null when nothing was received yet, so the withholding is unknown. */
  net_base: MoneyString | null;
}

export interface ExpectedDividendsData {
  as_of: DateString;
  to: DateString;
  base_currency: string;
  rows: ExpectedDividendRow[];
}

// --- Investment plans --------------------------------------------------------
