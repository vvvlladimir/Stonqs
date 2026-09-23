/** The ledger, and the round trips derived from it. */

import type { DateString, MoneyString } from "./primitives";
export type TransactionKind =
  | "BUY"
  | "SELL"
  | "DELIVERY_INBOUND"
  | "DELIVERY_OUTBOUND"
  | "SECURITY_TRANSFER_IN"
  | "SECURITY_TRANSFER_OUT"
  | "DIVIDEND"
  | "INTEREST"
  | "INTEREST_CHARGE"
  | "CASHBACK"
  | "REWARD"
  | "FEE"
  | "FEE_REFUND"
  | "TAX"
  | "TAX_REFUND"
  | "DEPOSIT"
  | "WITHDRAWAL"
  | "TRANSFER_IN"
  | "TRANSFER_OUT";

export interface Transaction {
  id: string;
  account_id: string;
  security_id: string | null;
  kind: TransactionKind;
  date: DateString;
  quantity: MoneyString;
  price: MoneyString;
  amount: MoneyString;
  fees: MoneyString;
  taxes: MoneyString;
  currency: string;
  /** Currency the commission was billed in; null when it is the operation's own. */
  fee_currency: string | null;
  tax_currency: string | null;
  /** Transaction-time FX rate, or null to use the rate table. */
  fx_rate_to_base: MoneyString | null;
  link_id: string | null;
  /** The broker's own identifier, for a row that came from a file. */
  external_id: string | null;
  note: string | null;
}

export interface TransactionRow extends Transaction {
  symbol: string | null;
  account_name: string;
  /** Transaction amount in base currency at its date's FX rate. */
  amount_base: MoneyString;
  /** Signed cash leg; purchases are negative. */
  net_base: MoneyString;
}

/**
 * Two stored operations that look like the two halves of one move between the user's own
 * accounts — the shape a portfolio carried from one broker to another leaves behind, where each
 * export knows only its own leg. A suggestion, never a decision: linking is the user's press.
 */
export interface TransferSuggestion {
  out_id: string;
  in_id: string;
  currency: string;
  amount_out: MoneyString;
  amount_in: MoneyString;
  date_out: DateString;
  date_in: DateString;
  account_out: string;
  account_in: string;
  account_out_name: string;
  account_in_name: string;
  /** Days between the two legs. */
  days_apart: number;
}

/** Monthly journal net computed by the core. */
export interface MonthlyNet {
  year: number;
  month: number;
  count: number;
  net_base: MoneyString;
}

/** Yearly journal net computed by the core. */
export interface YearlyNet {
  year: number;
  count: number;
  net_base: MoneyString;
}

export interface TransactionsData {
  base_currency: string;
  rows: TransactionRow[];
  /** Filtered rows in chronological order. */
  monthly_net: MonthlyNet[];
  yearly_net: YearlyNet[];
  total_net: MoneyString;
}

export interface TransactionFilter {
  account_id?: string | null;
  security_id?: string | null;
  kind?: TransactionKind | null;
  from?: DateString | null;
  to?: DateString | null;
}

export interface TransactionInput {
  id: string | null;
  account_id: string;
  security_id: string | null;
  kind: TransactionKind;
  date: DateString;
  quantity: string | null;
  price: string | null;
  amount: string | null;
  fees: string | null;
  taxes: string | null;
  currency: string;
  fee_currency: string | null;
  tax_currency: string | null;
  fx_rate_to_base: string | null;
  note: string | null;
}

// Performance and risk

/** One trade: the purchases behind a quantity and what leaving it brought in. */
export interface Trade {
  security_id: string;
  opened_at: DateString;
  /** The disposal that ended it; `null` while the shares are still held. */
  closed_at: DateString | null;
  quantity: MoneyString;
  entry_value_base: MoneyString;
  exit_value_base: MoneyString;
  pnl_base: MoneyString;
  /** Days held, weighted by quantity across the purchases. */
  holding_days: number;
  return_pct: MoneyString | null;
  irr: MoneyString | null;
}

export interface TradeRow extends Trade {
  symbol: string;
  name: string;
}

export interface TradeStats {
  trades: number;
  winners: number;
  losers: number;
  pnl_base: MoneyString;
  entry_value_base: MoneyString;
  exit_value_base: MoneyString;
  /** Holding period weighted by what each trade had invested. */
  average_holding_days: number;
  win_rate: MoneyString | null;
}

export interface TradingVolume {
  bought_base: MoneyString;
  sold_base: MoneyString;
  volume_base: MoneyString;
  trades: number;
}

export interface TradesData {
  from: DateString;
  to: DateString;
  base_currency: string;
  /** Open as of `to`; closed when the disposal fell inside the window. */
  open: TradeRow[];
  closed: TradeRow[];
  open_stats: TradeStats;
  closed_stats: TradeStats;
  volume: TradingVolume;
  turnover_rate: MoneyString | null;
}
