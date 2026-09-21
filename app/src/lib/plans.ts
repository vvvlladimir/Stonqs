import { today } from "./api";
import type { PlanInput, PlanRow, RebalanceTrade } from "./types";

/** What the editor edits, and how a plan is turned back into it. See ADR-0033. */

/** A new plan: monthly, starting today, buying nothing until instruments are added. */
export const EMPTY_PLAN: PlanInput = {
  id: null,
  account_id: "",
  name: "",
  amount: "",
  currency: "EUR",
  fees: "",
  taxes: "",
  start: today(),
  end: null,
  interval_unit: "MONTH",
  interval_count: 1,
  active: true,
  note: null,
  legs: [],
};

export function planToInput(row: PlanRow): PlanInput {
  const { plan } = row;
  return {
    id: plan.id,
    account_id: plan.account_id,
    name: plan.name,
    amount: plan.amount,
    currency: plan.currency,
    fees: plan.fees,
    taxes: plan.taxes,
    start: plan.schedule.start,
    end: plan.schedule.end,
    interval_unit: plan.schedule.unit,
    interval_count: plan.schedule.count,
    active: plan.active,
    note: plan.note,
    legs: plan.legs.map((leg) => ({ security_id: leg.security_id, weight: leg.weight })),
  };
}

/** A leg's share of the contribution: its weight over the sum of them. */
export function legShare(legs: PlanInput["legs"], index: number): number {
  const total = legs.reduce((sum, leg) => sum + amountOf(leg.weight), 0);
  return total > 0 ? amountOf(legs[index].weight) / total : 0;
}

/** A typed number, comma or dot; anything unreadable is zero rather than NaN. */
export function amountOf(value: string | null | undefined): number {
  const parsed = Number((value ?? "").replace(",", "."));
  return Number.isFinite(parsed) ? parsed : 0;
}

/**
 * A rebalance plan turned into a standing one: what the drift says to buy today becomes the
 * split of every future contribution. Sales are dropped — a plan pays money in, it does not
 * liquidate — and the amounts become the weights, so the proportions survive price moves.
 */
export function planFromTrades({
  name,
  accountId,
  currency,
  amount,
  trades,
}: {
  name: string;
  accountId: string;
  currency: string;
  amount: string;
  trades: RebalanceTrade[];
}): PlanInput {
  const buys = trades.filter((trade) => !trade.quantity.startsWith("-"));
  return {
    ...EMPTY_PLAN,
    name,
    account_id: accountId,
    currency,
    amount,
    start: today(),
    legs: buys.map((trade) => ({
      security_id: trade.security_id,
      // The estimated cost is the weight: rounding it to a percentage would lose the ratio.
      weight: trade.estimated_base,
    })),
  };
}
