/** Accounts, the groups they sit in, and the cash they hold. */

import type { DateString, MoneyString } from "./primitives";
export type AccountKind = "DEPOSIT" | "SECURITIES";

export interface Account {
  id: string;
  name: string;
  currency: string;
  kind: AccountKind;
  /** Cash account backing a securities account; null for cash accounts. */
  reference_account_id: string | null;
  is_active: boolean;
  opened_at: DateString | null;
}

/** Balance for one currency in an account. */
export interface CashBalance {
  currency: string;
  amount: MoneyString;
}

/** Account enriched with application-level summary fields. */
export interface AccountRow extends Account {
  transaction_count: number;
  in_portfolio: boolean;
  reference_name: string | null;
  balances: CashBalance[];
  /** Current securities value, or null when quotes are missing. */
  securities_value_base: MoneyString | null;
  /** Account card value in the base currency. */
  value_base: MoneyString | null;
}

export interface AccountInput {
  id: string | null;
  name: string;
  currency: string;
  kind: AccountKind;
  reference_account_id: string | null;
  is_active: boolean;
  opened_at: DateString | null;
}

export interface AccountGroup {
  id: string;
  name: string;
  account_ids: string[];
}

export interface AccountGroupRow extends AccountGroup {
  account_names: string[];
  /** Scope value in base currency, or null when market data is missing. */
  value_base: MoneyString | null;
}

/** Summary total for all portfolio accounts. */
export interface AccountsTotal {
  base_currency: string;
  value_base: MoneyString | null;
}

export interface AccountGroupInput {
  id: string | null;
  name: string;
  account_ids: string[];
}

/** Account fields required by the overview. */
export interface AccountRef {
  id: string;
  name: string;
  kind: AccountKind;
  currency: string;
}

/** One account balance in one currency. */
export interface AccountCash {
  account_id: string;
  currency: string;
  /** Decimal money serialized as a string. */
  amount: string;
}
