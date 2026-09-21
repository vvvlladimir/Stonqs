/** Target weights and the trades that would meet them. */

import type { MoneyString } from "./primitives";
export interface TargetWeight {
  node_id: string;
  /** Target weight relative to the parent node. */
  weight: MoneyString;
}

export interface AllocationTarget {
  id: string;
  portfolio_id: string;
  taxonomy_id: string;
  name: string;
  weights: TargetWeight[];
}

export interface RebalanceTrade {
  security_id: string;
  symbol: string;
  /** Signed quantity, rounded to the tradable step. */
  quantity: MoneyString;
  estimated_base: MoneyString;
  /** Base-currency price used to calculate the trade. */
  price_base: MoneyString;
  /** Post-plan portfolio weight across all trades for the security. */
  weight_after: MoneyString;
}

/** Cash assigned to a node; it is deposited rather than purchased. */
export interface CashDeposit {
  /** Cash key `cash:<account>:<currency>`. */
  subject_id: string;
  account_id: string;
  currency: string;
  /** Account name. */
  label: string;
  /** Current base-currency balance. */
  current_base: MoneyString;
  /** Base-currency amount to deposit or redirect to securities. */
  amount_base: MoneyString;
  /** Post-plan portfolio weight across all deposits. */
  weight_after: MoneyString;
}

export interface RebalanceItem {
  node_id: string;
  label: string;
  current_base: MoneyString;
  /** Weight of the whole portfolio. */
  current_weight: MoneyString;
  /** Portfolio target obtained from the path weights. */
  target_weight: MoneyString;
  /** User-defined target relative to the parent. */
  relative_target_weight: MoneyString;
  /** Current weight using the same denominator as the relative target. */
  relative_current_weight: MoneyString;
  target_base: MoneyString;
  drift_base: MoneyString;
  drift_weight: MoneyString;
  /** Leaf node where cash is allocated and trades are generated. */
  leaf: boolean;
  trades: RebalanceTrade[];
  /** Accounts receiving deposits for this node. */
  deposits: CashDeposit[];
}

export interface RebalancePlan {
  /** Portfolio value plus `cash_to_invest`, used as the target base. */
  total_base: MoneyString;
  items: RebalanceItem[];
  off_target_base: MoneyString;
  /** Purchase cash after rounding down to tradable steps. */
  cash_used_base: MoneyString;
  /** Sale proceeds without a sign; zero in buy-only mode. */
  sell_base: MoneyString;
  /** Remaining cash: deposits plus sales minus purchases. */
  cash_left_base: MoneyString;
}

// Reports
