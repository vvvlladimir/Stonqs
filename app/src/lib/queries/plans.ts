/** Plans, FIRE, goals and contribution limits. */

import { useQuery } from "@tanstack/react-query";
import { api } from "../api";
import type { DateString } from "../types";
import { keys } from "./keys";

export function usePlans() {
  return useQuery({ queryKey: keys.plans(), queryFn: api.plansList });
}

/** Disabled without a plan: an id is what makes the question answerable. */
export function usePlanDue(planId: string | null) {
  return useQuery({
    queryKey: keys.planDue(planId ?? undefined),
    queryFn: () => api.planDue(planId!),
    enabled: planId !== null,
  });
}

export function usePlanProjection(months: number) {
  return useQuery({
    queryKey: keys.planProjection(months),
    queryFn: () => api.planProjection(months),
  });
}

/** The assumptions are the key: two tiles asking different questions are two queries. */
export function useFire(assumptions: {
  annual_spending: string;
  withdrawal_rate: string;
  expected_return: string;
  contribution: string | null;
}) {
  const { annual_spending, withdrawal_rate, expected_return, contribution } = assumptions;
  return useQuery({
    queryKey: keys.fire(annual_spending, withdrawal_rate, expected_return, contribution),
    queryFn: () => api.fireProjection(assumptions),
    enabled: annual_spending !== "" && withdrawal_rate !== "" && expected_return !== "",
  });
}

/** Goals with their progress, read at `date`. Not scoped: a goal carries its own accounts. */
export function useGoals(date: DateString) {
  return useQuery({ queryKey: keys.goals(date), queryFn: () => api.goalsList(date) });
}

/** Contribution limits with what has been paid in over the limit year `date` falls in. */
export function useLimits(date: DateString) {
  return useQuery({ queryKey: keys.limits(date), queryFn: () => api.limitsList(date) });
}
