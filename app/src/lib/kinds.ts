import type { I18n, MessageDescriptor } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";
import type {
  AccountKind,
  AlertDirection,
  AlertKind,
  AttributeKind,
  DividendFrequency,
  PaymentLine,
  PlanInterval,
  SecurityEventKind,
  SecurityKind,
  TransactionKind,
} from "./types";

/** Central labels and ordering for model enum values. */

export const SECURITY_KIND_LABELS: Record<SecurityKind, MessageDescriptor> = {
  ETF: msg`ETF`,
  STOCK: msg`Stock`,
  BOND: msg`Bond`,
  FUND: msg`Fund`,
  CRYPTO: msg`Crypto`,
  OTHER: msg`Other`,
};

/** Short labels used by transaction filters. */
export const SECURITY_KIND_FILTERS: Record<SecurityKind, MessageDescriptor> = {
  ETF: msg`ETFs`,
  STOCK: msg`Stocks`,
  BOND: msg`Bonds`,
  FUND: msg`Funds`,
  CRYPTO: msg`Crypto assets`,
  OTHER: msg`Other assets`,
};

export const SECURITY_KINDS: SecurityKind[] = ["ETF", "STOCK", "BOND", "FUND", "CRYPTO", "OTHER"];

/** What an instrument attribute holds; the kind decides how its value is parsed. */
export const ATTRIBUTE_KIND_LABELS: Record<AttributeKind, MessageDescriptor> = {
  TEXT: msg`Text`,
  NUMBER: msg`Number`,
  DATE: msg`Date`,
};

export const ATTRIBUTE_KINDS: AttributeKind[] = ["TEXT", "NUMBER", "DATE"];

export function attributeKindLabel(i18n: I18n, kind: AttributeKind): string {
  return i18n._(ATTRIBUTE_KIND_LABELS[kind]);
}

/** What a rule on an instrument watches. */
export const ALERT_KIND_LABELS: Record<AlertKind, MessageDescriptor> = {
  PRICE: msg`Price level`,
  DATE_REACHED: msg`Date`,
};

export const ALERT_KINDS: AlertKind[] = ["PRICE", "DATE_REACHED"];

/** Which crossings of its level a price trigger announces. */
export const ALERT_DIRECTION_LABELS: Record<AlertDirection, MessageDescriptor> = {
  UP: msg`Rises above`,
  DOWN: msg`Falls below`,
  BOTH: msg`Crosses either way`,
};

export const ALERT_DIRECTIONS: AlertDirection[] = ["UP", "DOWN", "BOTH"];

export function alertDirectionLabel(i18n: I18n, direction: AlertDirection): string {
  return i18n._(ALERT_DIRECTION_LABELS[direction]);
}

export function alertKindLabel(i18n: I18n, kind: AlertKind): string {
  return i18n._(ALERT_KIND_LABELS[kind]);
}

export const SECURITY_EVENT_KIND_LABELS: Record<SecurityEventKind, MessageDescriptor> = {
  NOTE: msg`Note`,
  DIVIDEND: msg`Dividend`,
  SPLIT: msg`Split`,
};

export function securityEventKindLabel(i18n: I18n, kind: SecurityEventKind): string {
  return i18n._(SECURITY_EVENT_KIND_LABELS[kind]);
}

/** How often an instrument pays, as the core inferred it from the payment dates. `UNKNOWN`
 * has no label on purpose: fewer than two payments is an absent figure, not a word. */
export const DIVIDEND_FREQUENCY_LABELS: Partial<Record<DividendFrequency, MessageDescriptor>> = {
  MONTHLY: msg`Monthly`,
  QUARTERLY: msg`Quarterly`,
  SEMI_ANNUAL: msg`Twice a year`,
  ANNUAL: msg`Once a year`,
  IRREGULAR: msg`Irregular`,
};

/** The rows of the payments grid, in the order the core emits them. */
export const PAYMENT_LINE_LABELS: Record<PaymentLine, MessageDescriptor> = {
  DIVIDENDS: msg`Dividends`,
  INTEREST: msg`Interest`,
  INTEREST_CHARGE: msg`Interest charged`,
  OTHER_INCOME: msg`Other income`,
  FEES: msg`Fees`,
  TAXES: msg`Taxes`,
  SAVINGS: msg`Paid in`,
  CLOSED_TRADES: msg`Closed trades`,
};

export const ACCOUNT_KIND_LABELS: Record<AccountKind, MessageDescriptor> = {
  DEPOSIT: msg`Cash`,
  SECURITIES: msg`Securities`,
};

export const TRANSACTION_KIND_LABELS: Partial<Record<TransactionKind, MessageDescriptor>> = {
  BUY: msg`Buy`,
  SELL: msg`Sell`,
  DIVIDEND: msg`Dividend`,
  DEPOSIT: msg`Deposit`,
  WITHDRAWAL: msg`Withdrawal`,
  INTEREST: msg`Interest earned`,
  INTEREST_CHARGE: msg`Interest paid`,
  CASHBACK: msg`Cashback`,
  REWARD: msg`Holding reward`,
  FEE: msg`Fee`,
  FEE_REFUND: msg`Fee refund`,
  TAX: msg`Tax`,
  TAX_REFUND: msg`Tax refund`,
  DELIVERY_INBOUND: msg`Securities received`,
  DELIVERY_OUTBOUND: msg`Securities delivered`,
  TRANSFER_IN: msg`Cash transfer in`,
  TRANSFER_OUT: msg`Cash transfer out`,
  SECURITY_TRANSFER_IN: msg`Securities transfer in`,
  SECURITY_TRANSFER_OUT: msg`Securities transfer out`,
};

/** Transaction kinds exposed by forms; paired transfer records are excluded. */
export const EDITABLE_TRANSACTION_KINDS: TransactionKind[] = [
  "BUY",
  "SELL",
  "DIVIDEND",
  "DEPOSIT",
  "WITHDRAWAL",
  "INTEREST",
  "INTEREST_CHARGE",
  "CASHBACK",
  "REWARD",
  "FEE",
  "FEE_REFUND",
  "TAX",
  "TAX_REFUND",
  "DELIVERY_INBOUND",
  "DELIVERY_OUTBOUND",
];

/** A kind a newer host knows and this build does not falls back to the wire value. */
export function transactionLabel(i18n: I18n, kind: TransactionKind): string {
  const label = TRANSACTION_KIND_LABELS[kind];
  return label ? i18n._(label) : kind;
}

export function securityKindLabel(i18n: I18n, kind: SecurityKind): string {
  return i18n._(SECURITY_KIND_LABELS[kind]);
}

export function accountKindLabel(i18n: I18n, kind: AccountKind): string {
  return i18n._(ACCOUNT_KIND_LABELS[kind]);
}

/** Kinds the import wizard can map a broker column onto, in the order it offers them. */
export const IMPORTABLE_TRANSACTION_KINDS: TransactionKind[] = [
  "BUY",
  "SELL",
  "DIVIDEND",
  "INTEREST",
  "INTEREST_CHARGE",
  "CASHBACK",
  "REWARD",
  "DEPOSIT",
  "WITHDRAWAL",
  "FEE",
  "FEE_REFUND",
  "TAX",
  "TAX_REFUND",
  "TRANSFER_IN",
  "TRANSFER_OUT",
  "DELIVERY_INBOUND",
  "DELIVERY_OUTBOUND",
  "SECURITY_TRANSFER_IN",
  "SECURITY_TRANSFER_OUT",
];

/**
 * How often a plan fires. The core stores a unit and a count; the strip offers the cadences
 * people actually name, so "every 3 months" reads as "Quarterly" without the core knowing
 * the word. A stored pair outside the strip keeps its own wording.
 */
export const PLAN_CADENCES: Array<{ unit: PlanInterval; count: number; label: MessageDescriptor }> = [
  { unit: "WEEK", count: 1, label: msg`Weekly` },
  { unit: "WEEK", count: 2, label: msg`Every two weeks` },
  { unit: "MONTH", count: 1, label: msg`Monthly` },
  { unit: "MONTH", count: 3, label: msg`Quarterly` },
  { unit: "MONTH", count: 6, label: msg`Twice a year` },
  { unit: "MONTH", count: 12, label: msg`Yearly` },
];

export function planCadenceLabel(i18n: I18n, unit: PlanInterval, count: number): string {
  const named = PLAN_CADENCES.find((c) => c.unit === unit && c.count === count);
  if (named) return i18n._(named.label);
  // `plural` resolves against the active catalog itself, so it is already a sentence.
  return unit === "WEEK"
    ? plural(count, { one: "Every week", other: "Every # weeks" })
    : plural(count, { one: "Every month", other: "Every # months" });
}

/** A provider's own name, written the way the company writes it — never translated, and never a
 * sentence the app authored, which is why it sits apart from every label table above. An id this
 * build has no name for is shown exactly as it is stored. */
export function providerName(id: string, label?: string): string {
  // The user's own name for their server wins over anything written here.
  if (label?.trim()) return label.trim();
  // eslint-disable-next-line lingui/no-unlocalized-strings -- brand names, not UI text
  const NAMES: Record<string, string> = { openai: "OpenAI", anthropic: "Anthropic", gemini: "Gemini" };
  return NAMES[id] ?? id;
}

/** The id of the provider the user configures themselves — `ai::catalog::CUSTOM`. */
export const CUSTOM_PROVIDER = "custom";
