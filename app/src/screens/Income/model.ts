import type { I18n } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";
import { formatMoney, formatMonth, formatMonthShort } from "../../lib/format";
import { transactionLabel } from "../../lib/kinds";
import { slotFor } from "../../lib/plot";
import type { CalendarCell } from "../../components/charts";
import type { IncomeData, MonthIncome, PaymentBucket, PaymentPeriod, TransactionKind } from "../../lib/types";

/** Kinds the core counts as income; `all` keeps every one of them. */
export const INCOME_KINDS: TransactionKind[] = ["DIVIDEND", "INTEREST", "INTEREST_CHARGE"];

/** The core narrows the report itself — the UI never re-sums money. */
export function kindOptions(i18n: I18n) {
  return [
    { value: "all", label: i18n._(msg`All income kinds`) },
    ...INCOME_KINDS.map((kind) => ({
      value: kind,
      label: i18n._(msg`Only ${transactionLabel(i18n, kind).toLowerCase()}`),
    })),
  ];
}

/** Money columns are sized in characters, so they hold the widest amount they print. */
export const WIDTH = { money: "11ch", count: "6ch", payer: "12ch", share: "9ch" };

export function kindNote(i18n: I18n, kind: TransactionKind | null): string {
  return kind ? transactionLabel(i18n, kind).toLowerCase() : i18n._(msg`all kinds`);
}

/** The picked month, or the most recent one with a payment. */
export function pickMonth(
  months: MonthIncome[],
  picked: { year: number; month: number } | null,
): MonthIncome | null {
  if (months.length === 0) return null;
  const found = picked && months.find((m) => m.year === picked.year && m.month === picked.month);
  return found ?? months[months.length - 1];
}

export function monthCell(i18n: I18n, month: MonthIncome, currency: string): CalendarCell {
  return {
    year: month.year,
    month: month.month,
    value: Number(month.net_base),
    text: formatMoney(month.net_base, currency, {
      compact: true,
      symbol: false,
    }),
    title: i18n._(
      msg`${formatMonth(month.year, month.month)}: ${formatMoney(month.net_base, currency)} · ${plural(month.events, { one: "# payment", other: "# payments" })}`,
    ),
  };
}

/** Divide display values only to derive a visual share, never a total. */
export function ratio(part: string, whole: string): string {
  const total = Number(whole);
  return total === 0 ? "0" : String(Number(part) / total);
}

export function share(part: string, whole: string): string {
  return `${Math.max(Number(ratio(part, whole)) * 100, 0)}%`;
}

/** One palette slot (1-8) per kind, shared by the legend, the year bars and the table. */
export function kindSlots(all: IncomeData): Array<{ kind: TransactionKind; slot: number }> {
  const seen: TransactionKind[] = [];
  for (const row of all.by_year_kind) if (!seen.includes(row.kind)) seen.push(row.kind);
  for (const row of all.by_kind) if (!seen.includes(row.kind)) seen.push(row.kind);
  return seen.map((kind, i) => ({ kind, slot: slotFor(i) }));
}

/** Market value per security, used only to show a display-only yield. */
export function valueBySecurity(
  rows?: Array<{ security_id: string; market_value_base: string }>,
): Map<string, string> {
  return new Map((rows ?? []).map((row) => [row.security_id, row.market_value_base]));
}

/** Column widths of the payments grid: the axis scrolls, so a column states its own width. */
export function paymentPeriods(i18n: I18n) {
  return [
    { value: "MONTH" as const, label: i18n._(msg`Month`) },
    { value: "QUARTER" as const, label: i18n._(msg`Quarter`) },
    { value: "YEAR" as const, label: i18n._(msg`Year`) },
  ];
}

/** A bucket is an interval, not a label: the core dates it and the name is written here. */
export function bucketLabel(i18n: I18n, period: PaymentPeriod, bucket: PaymentBucket): string {
  const short = String(bucket.year).slice(2);
  if (period === "YEAR") return String(bucket.year);
  if (period === "QUARTER") return i18n._(msg`Q${bucket.index} ${short}`);
  return i18n._(msg`${formatMonthShort(bucket.index)} ${short}`);
}
