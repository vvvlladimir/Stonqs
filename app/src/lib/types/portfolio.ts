/** The portfolio itself, its setup, and what the app reports about its own state. */

import type { AccountCash, AccountRef } from "./accounts";
import type { PortfolioValuation } from "./positions";
import type { DateString } from "./primitives";
import type { SecurityRef } from "./securities";
export type CostBasisMethod = "FIFO" | "AVERAGE_COST";

export interface Portfolio {
  id: string;
  name: string;
  base_currency: string;
  account_ids: string[];
  cost_basis_method: CostBasisMethod;
}

export interface PortfolioInput {
  name: string;
  base_currency: string;
  cost_basis_method: CostBasisMethod;
  account_ids: string[];
}

export interface SetupInput {
  portfolio_name: string;
  base_currency: string;
  /** Cash account that starts the portfolio. */
  account_name: string;
  account_currency: string;
  /** Optional securities account linked to it. */
  securities_account_name: string | null;
}

/** Complete data payload for the overview screen. */
export interface DashboardData {
  valuation: PortfolioValuation;
  securities: SecurityRef[];
  /** Accounts included in the selected scope. */
  accounts: AccountRef[];
  cash: AccountCash[];
}

export interface AppStatus {
  portfolio_name: string;
  base_currency: string;
  /** Null before the first transaction. */
  inception: DateString | null;
  /** Zero means the onboarding flow should be shown. */
  account_count: number;
  db_path: string;
  dev_build: boolean;
  /** OS language as a BCP-47 tag, or null when the OS does not report one. */
  system_locale: string | null;
}
