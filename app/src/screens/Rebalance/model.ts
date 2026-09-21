import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { slotOfNode } from "../../lib/taxonomy";
import type {
  AccountCash,
  AccountRef,
  RebalanceItem,
  RebalanceTrade,
  SecurityRow,
  TaxonomyData,
} from "../../lib/types";

/** Preset deposit amounts replace the field value; money stays in core strings. */
export const PRESETS = ["250", "500", "1000", "2500"];

/** One table, two subjects: a security is bought, a balance is paid in. */
export type PlanRow =
  | { kind: "trade"; key: string; trade: RebalanceTrade; item: RebalanceItem }
  | { kind: "cash"; key: string; cash: CashRow };

export interface CashRow {
  key: string;
  account_name: string;
  currency: string;
  deposit: string | null;
  weight_after: string | null;
  node: string | null;
  slot: number;
}

export function cashInPlan(
  items: RebalanceItem[],
  taxonomy: TaxonomyData | undefined,
  balances: AccountCash[] | undefined,
  accounts: AccountRef[] | undefined,
  baseCurrency: string,
): CashRow[] {
  const rows: CashRow[] = [];
  const seen = new Set<string>();

  for (const item of items) {
    for (const deposit of item.deposits) {
      seen.add(deposit.subject_id);
      rows.push({
        key: `${deposit.subject_id}-${item.node_id}`,
        account_name: deposit.label,
        currency: baseCurrency,
        deposit: deposit.amount_base,
        weight_after: deposit.weight_after,
        node: item.label,
        slot: slotOfNode(taxonomy, item.node_id) ?? 1,
      });
    }
  }

  const nameOfAccount = (id: string) => accounts?.find((a) => a.id === id)?.name ?? id;
  for (const balance of balances ?? []) {
    const subject = `cash:${balance.account_id}:${balance.currency}`;
    if (seen.has(subject)) continue;
    if (taxonomy?.excluded.includes(subject)) continue;
    const part = taxonomy?.classifications.find((c) => c.subject_id === subject);
    const node = taxonomy?.nodes.find((n) => n.id === part?.node_id);
    rows.push({
      key: subject,
      account_name: nameOfAccount(balance.account_id),
      currency: balance.currency,
      deposit: null,
      weight_after: null,
      node: node?.name ?? null,
      slot: part ? (slotOfNode(taxonomy, part.node_id) ?? 1) : 1,
    });
  }
  return rows;
}

export function largestDrift(items: RebalanceItem[]): { item: RebalanceItem; points: number } | null {
  let worst: { item: RebalanceItem; points: number } | null = null;
  for (const item of items) {
    const points = (Number(item.current_weight) - Number(item.target_weight)) * 100;
    if (!worst || Math.abs(points) > Math.abs(worst.points)) worst = { item, points };
  }
  return worst;
}

export function formatPoints(i18n: I18n, points: number): string {
  const sign = points >= 0 ? "+" : "−";
  const size = Math.abs(points).toFixed(1);
  return i18n._(msg`${sign}${size} pp`);
}

export function nameOf(securities: SecurityRow[] | undefined, id: string, symbol: string): string {
  return securities?.find((s) => s.id === id)?.name || symbol;
}
