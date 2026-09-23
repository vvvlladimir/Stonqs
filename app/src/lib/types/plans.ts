/** Investment plans, what they are due to buy, and what they project. */

import type { DateString, MoneyString } from "./primitives";
import type { Transaction } from "./transactions";
/** Recurrence unit of a plan. Quarterly is `MONTH` with a count of 3 — see ADR-0033. */
export type PlanInterval = "WEEK" | "MONTH";

export interface PlanSchedule {
  start: DateString;
  end: DateString | null;
  unit: PlanInterval;
  count: number;
}

/** A leg's `weight` is read against the sum of the plan's weights, never against 1. */
export interface PlanLeg {
  security_id: string;
  weight: MoneyString;
}

export interface InvestmentPlan {
  id: string;
  portfolio_id: string;
  /** Securities account for a plan with legs, cash account for a contribution plan. */
  account_id: string;
  name: string;
  amount: MoneyString;
  currency: string;
  fees: MoneyString;
  taxes: MoneyString;
  schedule: PlanSchedule;
  active: boolean;
  note: string | null;
  legs: PlanLeg[];
}

export interface PlanLegRow {
  security_id: string;
  symbol: string;
  name: string;
  weight: MoneyString;
  /** The share the money is actually split by: `weight` over the plan's total. */
  share: MoneyString;
}

export interface PlanRow {
  plan: InvestmentPlan;
  account_name: string;
  legs: PlanLegRow[];
  next_date: DateString | null;
  due_count: number;
  last_executed: DateString | null;
}

export interface PlansData {
  base_currency: string;
  rows: PlanRow[];
  monthly_base: MoneyString;
}

export interface PlannedTrade {
  security_id: string;
  symbol: string;
  quantity: MoneyString;
  price: MoneyString;
  /** The listing's currency, not the plan's. */
  currency: string;
  amount: MoneyString;
  budget: MoneyString;
  fees: MoneyString;
  taxes: MoneyString;
  cash_left: MoneyString;
  fx_rate: MoneyString;
}

export interface PlanOccurrence {
  plan_id: string;
  date: DateString;
  account_id: string;
  amount: MoneyString;
  currency: string;
  trades: PlannedTrade[];
  cash_left: MoneyString;
}

/** Why a due occurrence has no draft. A code, not a sentence — the wording is ours. */
export type PlanProblem = "MISSING_PRICE" | "MISSING_RATE";

export interface PlanDue {
  date: DateString;
  occurrence: PlanOccurrence | null;
  drafts: Transaction[];
  problem: PlanProblem | null;
}

export interface Contribution {
  date: DateString;
  plan_id: string;
  plan_name: string;
  amount: MoneyString;
  currency: string;
  amount_base: MoneyString;
}

export interface PlanProjection {
  base_currency: string;
  contributions: Contribution[];
  /** `YYYY-MM` -> base-currency total. */
  by_month: Record<string, MoneyString>;
  total_base: MoneyString;
}

export interface PlanLegInput {
  security_id: string;
  weight: string;
}

export interface PlanInput {
  id?: string | null;
  account_id: string;
  name: string;
  amount: string;
  currency: string;
  fees?: string | null;
  taxes?: string | null;
  start: DateString;
  end?: DateString | null;
  interval_unit: PlanInterval;
  interval_count: number;
  active: boolean;
  note?: string | null;
  legs: PlanLegInput[];
}

// Watchlists

/** How far the portfolio is from paying for a year, under assumptions the user typed. */
export interface FireProjection {
  base_currency: string;
  /** Capital the yearly spending needs at the chosen withdrawal rate. */
  target_base: MoneyString;
  current_base: MoneyString;
  /** Still missing; zero once the target is met. */
  missing_base: MoneyString;
  /** Current over target; above one when the target is passed. */
  progress: MoneyString;
  /** What today's capital would sustain in a year at the same rate. */
  sustainable_annual_base: MoneyString;
  /** Null when this pace does not reach the target within a hundred years. */
  months_to_target: number | null;
  target_date: DateString | null;
  monthly_contribution_base: MoneyString;
  expected_return: MoneyString;
  withdrawal_rate: MoneyString;
}

/** An amount the user means to have by a date, over the accounts they name. */
export interface Goal {
  id: string;
  name: string;
  target_amount: MoneyString;
  currency: string;
  target_date: DateString | null;
  monthly_amount: MoneyString | null;
  /** The user's assumption as a fraction; `0.05` is 5% a year. Never the measured return. */
  expected_return: MoneyString;
  note: string | null;
  created_at: DateString;
  /** Accounts that count towards it; empty is the whole portfolio. */
  accounts: string[];
}

export interface GoalProgress {
  goal_id: string;
  name: string;
  target_base: MoneyString;
  current_base: MoneyString;
  missing_base: MoneyString;
  progress: MoneyString;
  /** Whole months to the target date; `null` without one, never negative. */
  months_left: number | null;
  /** What must be paid in monthly to arrive on time; `null` when the goal names no date. */
  required_monthly_base: MoneyString | null;
  /** Months until the stated monthly amount arrives; `null` when none is stated. */
  months_to_target: number | null;
  projected_date: DateString | null;
  /** `null` when either side of the comparison is missing — not a "no". */
  on_track: boolean | null;
  monthly_base: MoneyString | null;
  expected_return: MoneyString;
}

export interface GoalRow {
  goal: Goal;
  account_names: string[];
  progress: GoalProgress;
}

export interface GoalInput {
  id: string | null;
  name: string;
  target_amount: string;
  currency: string;
  target_date: string | null;
  monthly_amount: string | null;
  /** Percent as typed: `5` is five percent a year. */
  expected_return: string | null;
  note: string | null;
  accounts: string[];
}

/** What one account may take in one limit year, and what it has taken. */
export interface LimitUsage {
  limit_id: string;
  account_id: string;
  name: string;
  from: DateString;
  to: DateString;
  allowance: MoneyString;
  /** Paid in over the year — net of withdrawals only when they restore allowance — never below zero. */
  used: MoneyString;
  remaining: MoneyString;
  share: MoneyString;
  currency: string;
  withdrawals_restore: boolean;
}

export interface LimitInput {
  id: string | null;
  account_id: string;
  name: string;
  amount: string;
  currency: string;
  /** `MM-DD`: the day the limit year opens. */
  year_starts_on: string;
  /** A withdrawal gives allowance back (a "flexible" ISA). */
  withdrawals_restore: boolean;
  note: string | null;
}
