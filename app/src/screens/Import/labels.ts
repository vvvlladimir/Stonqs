import type { I18n, MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { IMPORTABLE_TRANSACTION_KINDS, transactionLabel } from "../../lib/kinds";
import type { BadgeTone } from "../../components/ui";
import type {
  AmountSign,
  ImportProblem,
  ImportField,
  ImportMapping,
  ImportPreviewData,
  ProblemCode,
  RowStatus,
  TransactionKind,
} from "../../lib/types";

/** Each step is one question, and its hint is the answer the step is asking for. */
export const STEPS: Array<{ title: MessageDescriptor; hint: MessageDescriptor }> = [
  { title: msg`File`, hint: msg`Which export we are reading and what is actually inside it.` },
  {
    title: msg`Parsing`,
    hint: msg`Explain the file's columns, its operation names and the account everything lands on.`,
  },
  {
    title: msg`Instruments`,
    hint: msg`Link the export's codes to real instruments so quotes can arrive.`,
  },
  {
    title: msg`Commit`,
    hint: msg`Look at the result and write it. Nothing in the database changes before that button.`,
  },
];

export const FIELDS: Array<[ImportField, MessageDescriptor]> = [
  ["DATE", msg`Date`],
  ["KIND", msg`Transaction kind`],
  ["SYMBOL", msg`Ticker`],
  ["ISIN", msg`ISIN`],
  ["NAME", msg`Instrument name`],
  ["QUANTITY", msg`Quantity`],
  ["PRICE", msg`Price`],
  ["AMOUNT", msg`Amount`],
  ["FEE", msg`Commission`],
  ["TAX", msg`Tax`],
  ["CURRENCY", msg`Currency`],
  ["FEE_CURRENCY", msg`Commission currency`],
  ["TAX_CURRENCY", msg`Tax currency`],
  ["FX_RATE", msg`FX rate`],
  ["ACCOUNT", msg`Account`],
  ["LINK_ID", msg`Link id`],
  ["NOTE", msg`Note`],
];

/** Fields required for parsing. */
export const REQUIRED_FIELDS: ImportField[] = ["DATE", "KIND"];

export function fieldLabel(i18n: I18n, field: ImportField): string {
  const found = FIELDS.find(([id]) => id === field);
  return found ? i18n._(found[1]) : field;
}

/** Required columns nobody has pointed at yet: the only hard block in the wizard. */
export function missingFields(mapping: ImportMapping | null): ImportField[] {
  if (!mapping) return REQUIRED_FIELDS;
  return REQUIRED_FIELDS.filter((field) => !mapping.columns[field]);
}

export function kinds(i18n: I18n): Array<[TransactionKind, string]> {
  return IMPORTABLE_TRANSACTION_KINDS.map((kind) => [kind, transactionLabel(i18n, kind)]);
}

export function kindLabels(i18n: I18n): Record<string, string> {
  return Object.fromEntries(kinds(i18n));
}

export const STATUS_LABELS: Record<RowStatus, MessageDescriptor> = {
  READY: msg`ready`,
  DUPLICATE: msg`duplicate`,
  UNKNOWN_SECURITY: msg`no instrument`,
  IGNORED: msg`skipped`,
  INVALID: msg`error`,
};

/** A row status is a state, so it is a badge tone rather than a colour class. */
export const STATUS_TONES: Record<RowStatus, BadgeTone> = {
  READY: "in",
  DUPLICATE: "neutral",
  UNKNOWN_SECURITY: "warn",
  IGNORED: "neutral",
  INVALID: "out",
};

export const SIGN_LABELS: Record<AmountSign, MessageDescriptor> = {
  SIGNED: msg`a minus means money out`,
  UNSIGNED: msg`direction comes from the transaction kind`,
};

/** Stable label for grouping parser notices. */
export const PROBLEM_LABELS: Record<ProblemCode, MessageDescriptor> = {
  ENCODING: msg`the file is not UTF-8`,
  MALFORMED_ROW: msg`the row could not be parsed`,
  MISSING_COLUMN: msg`a column is not assigned`,
  NOT_A_NUMBER: msg`not a number`,
  BAD_DATE: msg`the date could not be parsed`,
  MISSING_VALUE: msg`a required value is missing`,
  UNKNOWN_KIND: msg`the transaction kind is not mapped`,
  UNKNOWN_ACCOUNT: msg`the account is not mapped`,
  WRONG_ACCOUNT_KIND: msg`the wrong kind of account`,
  TRANSFER_WITH_SECURITY: msg`a transfer carrying an instrument`,
  INVALID_TRANSACTION: msg`the transaction failed validation`,
  DUPLICATE_IN_STORE: msg`already in the database`,
  DUPLICATE_IN_FILE: msg`repeated inside the file`,
  SECURITY_WITHOUT_SOURCE: msg`an instrument without a quote source`,
  DIRECTION_FROM_SIGN: msg`direction taken from the sign of the amount`,
  DIRECTION_CONFLICT: msg`the sign disagrees with the transaction kind`,
  AMOUNT_SIGN_AMBIGUOUS: msg`the sign of the amount only partly agrees with the kinds`,
  AMOUNT_VS_QUANTITY_PRICE: msg`the amount does not match quantity × price`,
  FEE_EXCEEDS_AMOUNT: msg`the commission exceeds the amount`,
  FX_RATE_ON_BASE_CURRENCY: msg`an FX rate on the base currency is not applied`,
  SINGLE_KIND_VALUE: msg`one transaction kind for the whole file`,
  FUTURE_DATE: msg`a date in the future`,
  IMPLAUSIBLE_DATE_SPAN: msg`the dates span decades`,
  ZERO_AMOUNT: msg`a zero amount`,
  SUSPICIOUS_CURRENCY: msg`an odd currency code`,
};

/** Column -> field: the reading direction of the mapping table, where a file column says what it is. */
export function fieldByColumn(mapping: ImportMapping): Record<string, ImportField> {
  const out: Record<string, ImportField> = {};
  for (const [field, column] of Object.entries(mapping.columns) as Array<[ImportField, string]>) {
    if (column) out[column] = field;
  }
  return out;
}

/** Point a file column at a field. A field lives in one column only, so the previous pair drops. */
export function assignColumn(mapping: ImportMapping, column: string, field: ImportField | ""): ImportMapping {
  const columns = { ...mapping.columns };
  for (const [f, c] of Object.entries(columns) as Array<[ImportField, string]>) {
    if (c === column) delete columns[f];
  }
  if (field) columns[field] = column;
  return { ...mapping, columns };
}

/**
 * What a file's operation value becomes. Beside the transaction kinds there is one more
 * answer, `SKIP`: a broker prints lines that are not operations at all, and the file must
 * not be held hostage by them.
 */
export const SKIP = "SKIP";

export type KindChoice = TransactionKind | typeof SKIP | "";

/** One decision covers every file value folded into the same line. */
export function assignKinds(mapping: ImportMapping, values: string[], kind: KindChoice): ImportMapping {
  const kind_aliases = { ...mapping.kind_aliases };
  const ignored = new Set(mapping.ignored_kinds);
  for (const value of values) {
    const key = normalizeAlias(value);
    // The two answers are exclusive: choosing a kind takes the value off the skip list.
    ignored.delete(key);
    delete kind_aliases[key];
    if (kind === SKIP) ignored.add(key);
    else if (kind) kind_aliases[key] = kind;
  }
  return { ...mapping, kind_aliases, ignored_kinds: [...ignored] };
}

export function assignAccounts(mapping: ImportMapping, values: string[], accountId: string): ImportMapping {
  const account_aliases = { ...mapping.account_aliases };
  for (const value of values) {
    const key = normalizeAlias(value);
    if (accountId) account_aliases[key] = accountId;
    else delete account_aliases[key];
  }
  return { ...mapping, account_aliases };
}

/** Normalize aliases using the same key as the core import service. */
export function normalizeAlias(value: string): string {
  return value
    .trim()
    .toUpperCase()
    .replace(/[\s_-]/g, "");
}

export type PreviewRow = ImportPreviewData["rows"][number];

/**
 * The sentence for a parser notice. The core sends a code and the values behind it, so the
 * wording lives here; `message` is the English fallback for a code we have no sentence for.
 */
export function problemDetail(i18n: I18n, problem: ImportProblem): string {
  const p = problem.params;
  if (!p) return problem.message;
  switch (problem.code) {
    case "AMOUNT_SIGN_AMBIGUOUS":
      return i18n._(
        msg`The sign of the amount agrees with the transaction direction in only ${p.percent} % of rows (${p.agree} of ${p.votes}). Direction is taken from the transaction kind; if it should come from the sign, check the kind mapping and the amount column.`,
      );
    case "AMOUNT_VS_QUANTITY_PRICE":
      return i18n._(
        msg`The amount ${p.amount} does not match quantity × price (${p.quantity} × ${p.price} = ${p.expected}). Check the columns and the decimal separator.`,
      );
    case "FEE_EXCEEDS_AMOUNT":
      return i18n._(
        msg`Commission and tax (${p.charges}) exceed the transaction amount (${p.amount}) — the columns look swapped.`,
      );
    case "FX_RATE_ON_BASE_CURRENCY":
      return i18n._(
        msg`An FX rate is given while the transaction currency ${p.base} equals the base currency, so it is not applied. This column usually holds the source currency's rate.`,
      );
    case "SUSPICIOUS_CURRENCY":
      return i18n._(msg`${p.currency} does not look like a currency code.`);
    case "FUTURE_DATE":
      return i18n._(msg`The date ${p.date} lies in the future — check the date format.`);
    case "SINGLE_KIND_VALUE":
      return i18n._(
        msg`All ${p.rows} rows carry the same transaction kind ${p.kind} — check that the right column is assigned to the kind.`,
      );
    case "IMPLAUSIBLE_DATE_SPAN":
      return i18n._(
        msg`The file's dates span ${p.min} to ${p.max} — the date format is most likely detected wrong.`,
      );
    default:
      return problem.message;
  }
}
