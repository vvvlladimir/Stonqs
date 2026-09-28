import type { I18n } from "@lingui/core";
import { today } from "../../lib/api";
import { toneClass } from "../../lib/format";
import { transactionLabel } from "../../lib/kinds";
import type { BadgeTone } from "../../components/ui";
import type {
  AccountRow,
  MonthlyNet,
  TransactionInput,
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

/** A new operation: a purchase on the first securities account, since a cash account holds none. */
export function blankTransaction(accounts: AccountRow[]): TransactionInput {
  const firstDepot = accounts.find((a) => a.kind === "SECURITIES");
  return {
    id: null,
    account_id: firstDepot?.id ?? accounts[0]?.id ?? "",
    security_id: null,
    kind: "BUY",
    date: today(),
    quantity: null,
    price: null,
    amount: null,
    fees: null,
    taxes: null,
    currency: firstDepot?.currency ?? accounts[0]?.currency ?? "EUR",
    fee_currency: null,
    tax_currency: null,
    fx_rate_to_base: null,
    note: null,
  };
}

export function draftOf(row: TransactionRow): TransactionInput {
  return {
    id: row.id,
    account_id: row.account_id,
    security_id: row.security_id,
    kind: row.kind,
    date: row.date,
    quantity: row.quantity,
    price: row.price,
    amount: row.amount,
    fees: row.fees,
    taxes: row.taxes,
    currency: row.currency,
    fee_currency: row.fee_currency,
    tax_currency: row.tax_currency,
    fx_rate_to_base: row.fx_rate_to_base,
    note: row.note,
  };
}

/** The search box: ticker, note, operation or account, case-insensitive. */
export function matching(i18n: I18n, query: string) {
  const needle = query.trim().toLowerCase();
  return (row: TransactionRow) =>
    !needle ||
    (row.symbol ?? "").toLowerCase().includes(needle) ||
    (row.note ?? "").toLowerCase().includes(needle) ||
    transactionLabel(i18n, row.kind).toLowerCase().includes(needle) ||
    row.account_name.toLowerCase().includes(needle);
}

/** Newest first. */
export function yearsOf(rows: TransactionRow[]): string[] {
  return [...new Set(rows.map((row) => row.date.slice(0, 4)))].sort((a, b) => Number(b) - Number(a));
}

/** Each instrument the journal names, once. */
export function securitiesIn(rows: TransactionRow[]): Array<{ id: string; symbol: string }> {
  const list: Array<{ id: string; symbol: string }> = [];
  for (const row of rows) {
    if (row.security_id && !list.some((s) => s.id === row.security_id)) {
      list.push({ id: row.security_id, symbol: row.symbol ?? row.security_id });
    }
  }
  return list;
}
