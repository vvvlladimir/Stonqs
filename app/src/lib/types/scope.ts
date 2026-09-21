/** The data source a reporting call is answered in — the picker's lens, not a filter on the ledger. */

import type { AccountKind } from "./accounts";
/** Data scope whose transactions feed all calculations. */
export type ScopeKind = "PORTFOLIO" | "GROUP" | "ACCOUNT" | "ACCOUNT_WITH_CASH";

export interface DataScope {
  kind: ScopeKind;
  id: string | null;
}

export interface ScopeOption {
  kind: ScopeKind;
  id: string | null;
  /** Portfolio, group or account name; the wording around it belongs to the UI. */
  name: string;
  account_kind: AccountKind | null;
  /** Settlement account name, set only for `ACCOUNT_WITH_CASH`. */
  cash_name: string | null;
  account_count: number;
}

export interface ScopeState {
  scope: DataScope;
  options: ScopeOption[];
}
