import { toneClass } from "../../lib/format";
import type { BadgeTone } from "../../components/ui";
import type {
  MonthlyNet,
  TransactionKind,
  TransactionRow,
  TransactionsData,
  YearlyNet,
} from "../../lib/types";

export const QUANTITY_KINDS: TransactionKind[] = [
  "BUY",
  "SELL",
  "DELIVERY_INBOUND",
  "DELIVERY_OUTBOUND",
  "SECURITY_TRANSFER_IN",
  "SECURITY_TRANSFER_OUT",
];

export const SECURITY_KINDS: TransactionKind[] = [...QUANTITY_KINDS, "DIVIDEND"];

/** Operation direction follows the cash-flow sign, not the operation kind. */
export function badgeTone(net: string): BadgeTone {
  const tone = toneClass(net);
  return tone === "pos" ? "in" : tone === "neg" ? "out" : "neutral";
}

/** Group rows using the monthly and yearly totals already computed by core. */
export interface MonthGroup extends MonthlyNet {
  rows: TransactionRow[];
}

export interface YearGroup extends YearlyNet {
  months: MonthGroup[];
  /** All rows in a year, used by the year-level selection. */
  rows: TransactionRow[];
}

export function groupByYear(data: TransactionsData, rows: TransactionRow[]): YearGroup[] {
  const nets = new Map(data.monthly_net.map((m) => [`${m.year}-${m.month}`, m]));
  const yearNets = new Map(data.yearly_net.map((y) => [y.year, y]));
  const years: YearGroup[] = [];
  const byYear = new Map<number, YearGroup>();
  const byMonth = new Map<string, MonthGroup>();

  for (const row of rows) {
    const [year, month] = row.date.split("-").map(Number);
    let yearGroup = byYear.get(year);
    if (!yearGroup) {
      const net = yearNets.get(year);
      yearGroup = {
        year,
        count: net?.count ?? 0,
        net_base: net?.net_base ?? "0",
        months: [],
        rows: [],
      };
      byYear.set(year, yearGroup);
      years.push(yearGroup);
    }
    yearGroup.rows.push(row);

    const key = `${year}-${month}`;
    let group = byMonth.get(key);
    if (!group) {
      const net = nets.get(key);
      group = { year, month, count: net?.count ?? 0, net_base: net?.net_base ?? "0", rows: [] };
      byMonth.set(key, group);
      yearGroup.months.push(group);
    }
    group.rows.push(row);
  }
  return years;
}
