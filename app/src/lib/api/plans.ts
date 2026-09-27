/** Intentions about the portfolio: savings plans, FIRE, goals and contribution limits. */

import type {
  InvestmentPlan,
  PlanDue,
  PlanInput,
  PlanProjection,
  FireProjection,
  PlansData,
  Goal,
  GoalInput,
  GoalRow,
  LimitInput,
  LimitUsage,
  TransactionInput,
  DateString,
} from "../types";
import { call } from "./core";

export const plansApi = {
  /** Regular contributions. A plan proposes transactions; nothing is written until committed. */
  plansList: () => call<PlansData>("plans_list"),
  planSave: (input: PlanInput) => call<InvestmentPlan>("plan_save", { input }),
  planDelete: (id: string) => call<void>("plan_delete", { id }),
  /** Occurrences up to today with nothing committed against them, drafts included. */
  planDue: (plan_id: string, as_of?: DateString) =>
    call<PlanDue[]>("plan_due", { planId: plan_id, asOf: as_of ?? null }),
  /** Writes the confirmed rows and records the occurrence; returns how many were written. */
  planCommit: (plan_id: string, date: DateString, drafts: TransactionInput[]) =>
    call<number>("plan_commit", { planId: plan_id, date, drafts }),
  planProjection: (months: number) => call<PlanProjection>("plan_projection", { months }),
  /** The FIRE reading: every figure but today's value is an assumption the user typed. */
  fireProjection: (assumptions: {
    annual_spending: string;
    withdrawal_rate: string;
    expected_return: string;
    contribution: string | null;
  }) =>
    call<FireProjection>("fire_projection", {
      annualSpending: assumptions.annual_spending,
      withdrawalRate: assumptions.withdrawal_rate,
      expectedReturn: assumptions.expected_return,
      contribution: assumptions.contribution,
    }),

  /** Goals with their progress. Not scoped: a goal carries the accounts it counts. */
  goalsList: (date: DateString) => call<GoalRow[]>("goals_list", { date }),
  goalSave: (input: GoalInput) => call<Goal>("goal_save", { input }),
  goalDelete: (id: string) => call<void>("goal_delete", { id }),
  /** Contribution limits with what the limit year `date` falls in has taken. */
  limitsList: (date: DateString) => call<LimitUsage[]>("limits_list", { date }),
  limitSave: (input: LimitInput) => call<unknown>("limit_save", { input }),
  limitDelete: (id: string) => call<void>("limit_delete", { id }),
};
