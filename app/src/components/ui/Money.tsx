import type { ReactNode } from "react";
import {
  formatDecimal,
  formatMoney,
  formatMonthShort,
  formatPercent,
  formatQuantity,
  formatRate,
  formatStat,
  toneClass,
} from "../../lib/format";

/**
 * Values are rendered by these components, never by calling `format*` in markup:
 * the `num` class and the positive/negative colour belong to the value, not to
 * whoever happens to print it.
 */

interface ValueProps {
  /** Colour by sign; defaults to on whenever a sign is printed. */
  tone?: boolean;
  /** Secondary weight for supporting figures (counts, rates already explained). */
  dim?: boolean;
  className?: string;
  title?: string;
}

function Value({ text, value, tone, dim, className, title }: ValueProps & { text: string; value?: string }) {
  const classes = ["num"];
  if (tone && value !== undefined) {
    const sign = toneClass(value);
    if (sign) classes.push(sign);
  }
  if (dim) classes.push("dim");
  if (className) classes.push(className);
  return (
    <span className={classes.join(" ")} title={title}>
      {text}
    </span>
  );
}

export interface MoneyProps extends ValueProps {
  value: string;
  /** Omitted where the column header already names the currency. */
  currency?: string;
  digits?: number;
  signed?: boolean;
  compact?: boolean;
}

export function Money({ value, currency, digits, signed, compact, tone, ...rest }: MoneyProps) {
  const text =
    currency === undefined
      ? formatDecimal(value, { digits, signed })
      : formatMoney(value, currency, { digits, signed, compact });
  return <Value text={text} value={value} tone={tone ?? signed} {...rest} />;
}

export interface PercentProps extends ValueProps {
  /** A fraction as a decimal string; scaling happens in `lib/format`. */
  value: string;
  digits?: number;
  signed?: boolean;
}

export function Percent({ value, digits, signed, tone, ...rest }: PercentProps) {
  return (
    <Value text={formatPercent(value, { digits, signed })} value={value} tone={tone ?? signed} {...rest} />
  );
}

/** Quantities carry up to eight decimals and never a currency. */
export function Quantity({ value, ...rest }: ValueProps & { value: string }) {
  return <Value text={formatQuantity(value)} {...rest} />;
}

/** A statistic computed as `f64` (volatility, drawdown depth), shown as a percentage. */
export function Rate({
  value,
  digits,
  signed,
  tone,
  ...rest
}: ValueProps & { value: number; digits?: number; signed?: boolean }) {
  // A rate carries its own sign, so it can be coloured by it like every other value.
  return (
    <Value text={formatRate(value, digits, signed)} value={String(value)} tone={tone ?? signed} {...rest} />
  );
}

/** A non-percentage statistic such as Sharpe. */
export function Stat({ value, digits, ...rest }: ValueProps & { value: number | null; digits?: number }) {
  return <Value text={formatStat(value, digits)} {...rest} />;
}

/** Plain counts and already-formatted figures that still want tabular figures. */
export function Num({ children, dim, className }: { children: ReactNode } & ValueProps) {
  return <span className={`num${dim ? " dim" : ""}${className ? ` ${className}` : ""}`}>{children}</span>;
}

/**
 * A date as a stacked block — day, month, year. A one-line date wraps into three lines in a
 * narrow table column; this shape is the same height whatever the column width is. A list that
 * only ever looks a few months ahead drops the year (`year={false}`): inside a dashboard tile
 * the third line costs a row of the list and says nothing the list does not already imply.
 */
export function DayMark({ date, year: withYear = true }: { date: string; year?: boolean }) {
  const [year, month, day] = date.split("-");
  if (!year || !month || !day) return <span className="daymark">{date}</span>;
  return (
    <span className="daymark">
      <b className="num">{Number(day)}</b>
      <span>{formatMonthShort(Number(month))}</span>
      {withYear && <span className="num">{year}</span>}
    </span>
  );
}
