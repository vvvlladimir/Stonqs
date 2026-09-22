import type { I18n, MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { IMPORTABLE_TRANSACTION_KINDS, accountKindLabel, transactionLabel } from "../../lib/kinds";
import type { BadgeTone } from "../../components/ui";
import type {
  AccountRow,
  AmountBasis,
  AmountSign,
  ImportRule,
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

/** An account named the way the wizard needs it: what kind it is, then which one, then in what. */
export function accountLabel(i18n: I18n, account: AccountRow): string {
  return `${accountKindLabel(i18n, account.kind)} · ${account.name} · ${account.currency}`;
}

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
  UPDATED: msg`restated`,
  UNKNOWN_SECURITY: msg`no instrument`,
  SIMILAR: msg`looks stored`,
  IGNORED: msg`skipped`,
  INVALID: msg`error`,
};

/** A row status is a state, so it is a badge tone rather than a colour class. */
export const STATUS_TONES: Record<RowStatus, BadgeTone> = {
  READY: "in",
  DUPLICATE: "neutral",
  UPDATED: "warn",
  UNKNOWN_SECURITY: "warn",
  SIMILAR: "warn",
  IGNORED: "neutral",
  INVALID: "out",
};

export const SIGN_LABELS: Record<AmountSign, MessageDescriptor> = {
  SIGNED: msg`a minus means money out`,
  UNSIGNED: msg`direction comes from the transaction kind`,
};

export const BASIS_LABELS: Record<AmountBasis, MessageDescriptor> = {
  GROSS: msg`the amount is the trade itself`,
  NET: msg`the amount includes commission and tax`,
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
  RESTATED_IN_STORE: msg`the broker restated a row already imported`,
  SECURITY_WITHOUT_SOURCE: msg`an instrument without a quote source`,
  DIRECTION_FROM_SIGN: msg`direction taken from the sign of the amount`,
  DIRECTION_CONFLICT: msg`the sign disagrees with the transaction kind`,
  AMOUNT_BASIS_AMBIGUOUS: msg`the amount reads as gross in some rows and as net in others`,
  AMOUNT_SIGN_AMBIGUOUS: msg`the sign of the amount only partly agrees with the kinds`,
  AMOUNT_VS_QUANTITY_PRICE: msg`the amount does not match quantity × price`,
  FEE_EXCEEDS_AMOUNT: msg`the commission exceeds the amount`,
  FX_RATE_ON_BASE_CURRENCY: msg`an FX rate on the base currency is not applied`,
  SINGLE_KIND_VALUE: msg`one transaction kind for the whole file`,
  FUTURE_DATE: msg`a date in the future`,
  IMPLAUSIBLE_DATE_SPAN: msg`the dates span decades`,
  ZERO_AMOUNT: msg`a zero amount`,
  SUSPICIOUS_CURRENCY: msg`an odd currency code`,
  DELIVERY_WITHOUT_COST: msg`shares moved with no value given`,
  ACCOUNT_CURRENCY_MISMATCH: msg`another currency than the account keeps`,
  TICKER_ISIN_CONFLICT: msg`the ticker already names another instrument`,
  SIMILAR_IN_STORE: msg`an operation like it is already stored`,
  POSSIBLE_SPLIT: msg`the prices step by a whole factor`,
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

/**
 * The two-operation answers. One broker line is sometimes two operations — a reinvested
 * dividend is an income and a purchase, a wallet move is a leg out and a leg in — and the
 * wizard offers those two beside the single kinds rather than a rule editor. Anything more
 * elaborate is written in the layout itself.
 */
export const SPLITS = {
  "SPLIT:DIVIDEND+BUY": {
    label: msg`a dividend and a purchase`,
    emit: [{ kind: "DIVIDEND" as const, set: { QUANTITY: "0" } }, { kind: "BUY" as const }],
    link: false,
  },
  "SPLIT:TRANSFER": {
    label: msg`two legs of one move`,
    emit: [{ kind: "TRANSFER_OUT" as const }, { kind: "TRANSFER_IN" as const }],
    link: true,
  },
};

export type SplitChoice = keyof typeof SPLITS;

export type KindChoice = TransactionKind | typeof SKIP | SplitChoice | "";

/** Whether an answer is one of the two-operation ones. */
export function isSplit(choice: KindChoice): choice is SplitChoice {
  return choice in SPLITS;
}

/** The split currently answering for this file value, if a rule says so. */
export function splitOf(mapping: ImportMapping, value: string): SplitChoice | null {
  const rule = ruleFor(mapping, value);
  if (!rule) return null;
  const kinds = rule.emit.map((e) => e.kind).join("+");
  const found = Object.entries(SPLITS).find(([, s]) => s.emit.map((e) => e.kind).join("+") === kinds);
  return (found?.[0] as SplitChoice) ?? null;
}

function ruleFor(mapping: ImportMapping, value: string): ImportRule | undefined {
  const key = normalizeAlias(value);
  return mapping.rules?.find((rule) =>
    rule.when.some((c) => c.field === "KIND" && "equals" in c && normalizeAlias(c.equals) === key),
  );
}

/** One decision covers every file value folded into the same line. */
export function assignKinds(mapping: ImportMapping, values: string[], kind: KindChoice): ImportMapping {
  const kind_aliases = { ...mapping.kind_aliases };
  const ignored = new Set(mapping.ignored_kinds);
  const keys = values.map(normalizeAlias);
  // Every answer is exclusive with the others: a value has one of them, never two.
  const rules = (mapping.rules ?? []).filter(
    (rule) =>
      !rule.when.some((c) => c.field === "KIND" && "equals" in c && keys.includes(normalizeAlias(c.equals))),
  );
  for (const value of values) {
    const key = normalizeAlias(value);
    ignored.delete(key);
    delete kind_aliases[key];
    if (kind === SKIP) ignored.add(key);
    else if (isSplit(kind)) {
      const split = SPLITS[kind];
      rules.push({ when: [{ field: "KIND", equals: value }], emit: split.emit, link: split.link });
    } else if (kind) kind_aliases[key] = kind;
  }
  return { ...mapping, kind_aliases, ignored_kinds: [...ignored], rules };
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
    case "AMOUNT_BASIS_AMBIGUOUS":
      return i18n._(
        msg`The amount matches quantity × price in ${p.gross} rows and the same total with commission and tax in ${p.net} — only ${p.percent} % agree. Set it by hand if the file means the other one.`,
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
    case "DELIVERY_WITHOUT_COST":
      return i18n._(
        msg`${p.quantity} of ${p.symbol} move with no value given. The lot enters at a cost of zero and the whole holding will read as profit — open the row and enter the price paid, or the total.`,
      );
    case "ACCOUNT_CURRENCY_MISMATCH":
      return i18n._(
        msg`The row is in ${p.currency} and the account it lands on keeps ${p.account}. Correct it if the currency column was read wrong; ignore it if the account really holds both.`,
      );
    case "TICKER_ISIN_CONFLICT":
      return i18n._(
        msg`Ticker ${p.symbol} is already in the database under ISIN ${p.stored}, and this row says ${p.isin} — two instruments cannot share one ticker. Give this one a ticker of its own on the "Instruments" step.`,
      );
    case "POSSIBLE_SPLIT":
      return i18n._(
        msg`${p.symbol} trades at ${p.before} and then at ${p.after} on ${p.date} — a factor of about ${p.ratio}. If the broker applied a split here, the quantities before and after mean different shares; record the split on the instrument instead of importing the change.`,
      );
    default:
      return problem.message;
  }
}
