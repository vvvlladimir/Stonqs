/** Formats money as decimal strings without converting it to JavaScript numbers. */

import { i18n, type MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { tickStep } from "./plot";
import { currentLocale, type Locale } from "./i18n";

const DECIMAL_RE = /^([+-]?)(\d*)(?:\.(\d*))?$/;

/** Numbers and dates follow the UI language, never the host OS locale on its own. */
const INTL_LOCALES: Record<Locale, string> = { en: "en-US", ru: "ru-RU" };

function intlLocale(): string {
  return INTL_LOCALES[currentLocale()];
}

interface Parts {
  sign: "" | "-";
  int: string;
  frac: string;
}

/** Half-up rounding performed directly on the decimal string. */
function roundDecimalString(value: string, digits: number): Parts {
  const match = DECIMAL_RE.exec(value.trim());
  if (!match) return { sign: "", int: "0", frac: "0".repeat(digits) };

  let sign: "" | "-" = match[1] === "-" ? "-" : "";
  const rawInt = match[2] || "0";
  const rawFrac = match[3] || "";

  if (rawFrac.length <= digits) {
    const int = rawInt.replace(/^0+(?=\d)/, "");
    const frac = rawFrac.padEnd(digits, "0");
    if (sign === "-" && /^0*$/.test(int + frac)) sign = "";
    return { sign, int, frac };
  }

  const kept = (rawInt + rawFrac.slice(0, digits)).split("");
  if (rawFrac.charCodeAt(digits) - 48 >= 5) {
    let i = kept.length - 1;
    for (; i >= 0; i--) {
      const next = kept[i].charCodeAt(0) - 48 + 1;
      if (next === 10) {
        kept[i] = "0";
      } else {
        kept[i] = String(next);
        break;
      }
    }
    if (i < 0) kept.unshift("1");
  }

  const joined = kept.join("");
  const cut = joined.length - digits;
  const int = (joined.slice(0, cut) || "0").replace(/^0+(?=\d)/, "");
  const frac = joined.slice(cut);
  if (sign === "-" && /^0*$/.test(int + frac)) sign = "";
  return { sign, int, frac };
}

/** Separators are resolved once per locale: `Intl` is slow enough to matter in a table. */
const separatorCache = new Map<string, { group: string; decimal: string }>();

function separatorsOf(): { group: string; decimal: string } {
  const locale = intlLocale();
  const cached = separatorCache.get(locale);
  if (cached) return cached;
  const parts = new Intl.NumberFormat(locale).formatToParts(12345.6);
  const resolved = {
    group: parts.find((p) => p.type === "group")?.value ?? " ",
    decimal: parts.find((p) => p.type === "decimal")?.value ?? ".",
  };
  separatorCache.set(locale, resolved);
  return resolved;
}

function groupDigits(int: string): string {
  const group = separatorsOf().group;
  let out = "";
  for (let i = 0; i < int.length; i++) {
    if (i > 0 && (int.length - i) % 3 === 0) out += group;
    out += int[i];
  }
  return out;
}

export interface MoneyFormatOptions {
  digits?: number;
  /** Show `+` for positive P/L values. */
  signed?: boolean;
}

export function formatDecimal(value: string, options: MoneyFormatOptions = {}): string {
  const digits = options.digits ?? 2;
  const { sign, int, frac } = roundDecimalString(value, digits);
  const plus = options.signed && sign === "" && !/^0*$/.test(int + frac) ? "+" : "";
  const body = digits === 0 ? groupDigits(int) : `${groupDigits(int)}${separatorsOf().decimal}${frac}`;
  return `${plus}${sign}${body}`;
}

export interface MoneyOptions extends MoneyFormatOptions {
  /** Use compact units for axes, tiles, and calendar cells. */
  compact?: boolean;
  /** Append the currency code unless the surrounding heading already has it. */
  symbol?: boolean;
}

export function formatMoney(value: string, currency: string, options: MoneyOptions = {}): string {
  const body = options.compact
    ? formatCompact(value, options.digits ?? 1, { signed: options.signed })
    : formatDecimal(value, options);
  return options.symbol === false ? body : `${body} ${currency}`;
}

/** Formats quantities with up to eight significant decimal places. */
export function formatQuantity(value: string): string {
  const trimmed = value.includes(".") ? value.replace(/0+$/, "").replace(/\.$/, "") : value;
  const decimals = trimmed.split(".")[1]?.length ?? 0;
  return formatDecimal(trimmed, { digits: Math.min(decimals, 8) });
}

/** Returns the semantic sign used for coloring. */
export function signOf(value: string): "positive" | "negative" | "zero" {
  const { sign, int, frac } = roundDecimalString(value, 2);
  if (/^0*$/.test(int + frac)) return "zero";
  return sign === "-" ? "negative" : "positive";
}

/** Formats a fractional value as a percentage without floating-point scaling. */
function shiftTwo(value: string): string {
  const match = DECIMAL_RE.exec(value.trim());
  if (!match) return "0";
  const sign = match[1] === "-" ? "-" : "";
  const int = match[2] || "0";
  const frac = (match[3] || "").padEnd(2, "0");
  return `${sign}${int}${frac.slice(0, 2)}.${frac.slice(2)}`;
}

export function formatPercent(value: string, options: MoneyFormatOptions = {}): string {
  const digits = options.digits ?? 2;
  return `${formatDecimal(shiftTwo(value), { ...options, digits })} %`;
}

/** Formats numeric statistical rates, which are not monetary values. */
export function formatRate(value: number, digits = 1, signed = false): string {
  if (!Number.isFinite(value)) return "—";
  return `${formatDecimal((value * 100).toFixed(digits + 2), { digits, signed })} %`;
}

/** Formats a non-percentage statistic such as Sharpe or a coefficient. */
export function formatStat(value: number | null, digits = 2): string {
  if (value === null || !Number.isFinite(value)) return "—";
  return formatDecimal(value.toFixed(digits + 2), { digits });
}

/** Formats large amounts with compact units without dividing the money value. */
const UNITS: [number, MessageDescriptor][] = [
  [10, msg`bn`],
  [7, msg`M`],
  [4, msg`k`],
];

export function formatCompact(value: string, digits = 1, options: { signed?: boolean } = {}): string {
  const { sign, int } = roundDecimalString(value, 0);
  const plus = options.signed && sign === "" && !/^0*$/.test(int) ? "+" : "";
  const unit = UNITS.find(([length]) => int.length >= length);
  if (!unit) return `${plus}${sign}${groupDigits(int)}`;
  const [length, label] = unit;
  const suffix = i18n._(label);
  const cut = length - 1; // Remove three, six, or nine digits for the unit.
  const head = int.slice(0, int.length - cut);
  const tail = int.slice(int.length - cut, int.length - cut + digits);
  const body = digits > 0 && tail ? `${head}${separatorsOf().decimal}${tail}` : head;
  return `${plus}${sign}${body} ${suffix}`;
}

/** One label style for a whole axis: the compact unit follows the largest tick and the
 *  decimals the gap between them, so two neighbouring gridlines never print the same text.
 *  `percent` is for axes holding a rate (0.05), which prints as 5 %. */
export function axisFormat(ticks: number[], options: { percent?: boolean } = {}): (value: number) => string {
  const step = tickStep(ticks);
  if (options.percent) {
    const digits = decimalsFor(step * 100);
    return (value) => formatRate(value, digits);
  }

  // One unit for the whole axis: `formatCompact` picks it per value, which would print
  // "500 k" next to "1.0 M". Ticks are plot floats already, so dividing one is not
  // money arithmetic.
  const peak = Math.max(...ticks.map(Math.abs), 0);
  const found = AXIS_UNITS.find(([size]) => peak >= size);
  const scale = found?.[0] ?? 1;
  const unit = found ? ` ${i18n._(found[1])}` : "";
  const digits = decimalsFor(step / scale);
  return (value) => `${formatDecimal(String(value / scale), { digits })}${unit}`;
}

/** Axis units by scale, matching the labels of [`formatCompact`]. */
const AXIS_UNITS: [number, MessageDescriptor][] = [
  [1e9, msg`bn`],
  [1e6, msg`M`],
  [1e3, msg`k`],
];

/** Decimals a value of this size needs before it stops rounding onto its neighbour. */
function decimalsFor(step: number): number {
  if (!Number.isFinite(step) || step <= 0) return 0;
  return step >= 1 ? 0 : Math.min(Math.ceil(-Math.log10(step)), 6);
}

/**
 * Formats a date and drops the marker some locales print after the year (Russian writes
 * "2024 г."). Editing the parts keeps this locale-agnostic: no language's wording is spelled
 * out here, and a locale that adds no marker passes through untouched.
 */
function formatDate(at: Date, options: Intl.DateTimeFormatOptions): string {
  const parts = new Intl.DateTimeFormat(intlLocale(), options).formatToParts(at);
  return parts
    .map((part, index) =>
      part.type === "literal" && parts[index - 1]?.type === "year"
        ? part.value.replace(/\p{L}+\.?/gu, "").replace(/\s+(?=[,;.])/gu, "")
        : part.value,
    )
    .join("")
    .trim();
}

/** Formats a month label for axes and group headings. */
export function formatMonth(year: number, month: number): string {
  return formatDate(new Date(year, month - 1, 1), { month: "short", year: "numeric" });
}

/** Formats a month name when the year is shown separately. */
export function formatMonthName(month: number): string {
  return new Intl.DateTimeFormat(intlLocale(), { month: "long" }).format(new Date(2000, month - 1, 1));
}

/** Formats a one-letter month initial for the densest calendar heads. */
export function formatMonthNarrow(month: number): string {
  return new Intl.DateTimeFormat(intlLocale(), { month: "narrow" }).format(new Date(2000, month - 1, 1));
}

/** Formats a short month name for narrow date columns. */
export function formatMonthShort(month: number): string {
  const label = new Intl.DateTimeFormat(intlLocale(), { month: "short" }).format(
    new Date(2000, month - 1, 1),
  );
  return label.replace(".", "");
}

/** Formats a compact month-and-year axis label. */
export function formatAxisDate(date: string): string {
  const [year, month] = date.split("-").map(Number);
  if (!year || !month) return date;
  const label = new Intl.DateTimeFormat(intlLocale(), { month: "short" }).format(
    new Date(year, month - 1, 1),
  );
  return `${label.replace(".", "")} ${String(year).slice(2)}`;
}

/** Formats an ISO timestamp for last-updated labels. */
export function formatDateTime(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return formatDate(at, {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Formats a `YYYY-MM-DD` day label. */
export function formatDay(date: string): string {
  const [year, month, day] = date.split("-").map(Number);
  if (!year || !month || !day) return date;
  return formatDate(new Date(year, month - 1, day), { day: "numeric", month: "short", year: "numeric" });
}

/** Maps a signed value to the shared positive/negative CSS classes. */
/** A decimal string as a number to order rows by: an absent value stays absent, never 0. */
export function toNumber(value: string | number | null | undefined): number | null {
  if (value === null || value === undefined || value === "") return null;
  return Number(value);
}

export function toneClass(value: string): string {
  const tone = signOf(value);
  return tone === "positive" ? "pos" : tone === "negative" ? "neg" : "";
}

/** Region names the two aggregates have no ISO code for; every other region comes from `Intl`. */
const AGGREGATE_REGIONS: Record<string, MessageDescriptor> = {
  EA: msg`Euro area`,
  EU: msg`European Union`,
};

/**
 * Names a price-index region. The host sends only the code, so the name is written here where
 * the language is known — `Intl` has every country in every locale the app ships.
 */
export function formatRegion(code: string): string {
  const aggregate = AGGREGATE_REGIONS[code];
  if (aggregate) return i18n._(aggregate);
  const names = new Intl.DisplayNames([intlLocale()], { type: "region" });
  return names.of(code) ?? code;
}
