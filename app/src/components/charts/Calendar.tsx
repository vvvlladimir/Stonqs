import { Trans, useLingui } from "@lingui/react/macro";
import { formatMonth, formatMonthNarrow, formatMonthShort } from "../../lib/format";
import type { ChartHeight } from "./Chart";

/** Renders monthly values in aligned year rows for seasonal comparison. */
export interface CalendarCell {
  year: number;
  month: number;
  /** Numeric value used only for signed color intensity. */
  value: number;
  /** Optional preformatted cell text. */
  text?: string;
  title: string;
}

interface Props {
  cells: CalendarCell[];
  /** Optional yearly totals shown in the final column. */
  totals?: Array<{ year: number; text: string }>;
  /** `"fill"` stretches the rows over the box the calendar is in; see `Chart`'s own height. */
  height?: ChartHeight;
  /** Wide reading layout: month names, taller cells, oldest year on top. */
  full?: boolean;
  /** Makes cells pickable; the picked month is marked with `aria-pressed`. */
  selected?: { year: number; month: number };
  onPick?: (year: number, month: number) => void;
  /** Bottom row: the same month summed across every year. */
  footer?: Array<{ month: number; text: string }>;
  /** Grand total shown where the footer row meets the year column. */
  footerTotal?: string;
}

const MONTH_NUMBERS = Array.from({ length: 12 }, (_, i) => i + 1);

export function Calendar({ cells, totals, full, height, selected, onPick, footer, footerTotal }: Props) {
  const { t } = useLingui();
  if (cells.length === 0)
    return (
      <p className="muted">
        <Trans>No data for this period.</Trans>
      </p>
    );

  // Full layout reads as a timeline, so the oldest year comes first there.
  const years = [...new Set(cells.map((c) => c.year))].sort((a, b) => (full ? a - b : b - a));
  const peak = Math.max(...cells.map((c) => Math.abs(c.value)), Number.EPSILON);
  const byKey = new Map(cells.map((c) => [`${c.year}-${c.month}`, c]));
  const totalOf = new Map((totals ?? []).map((t) => [t.year, t.text]));
  // Month heads come from Intl, so a language change renames them without a table here.
  const heads = MONTH_NUMBERS.map((m) => (full ? formatMonthShort(m) : formatMonthNarrow(m)));

  return (
    <div
      className={`cal${full ? " cal--full" : ""}${height === "fill" ? " cal--fill" : ""}`}
      role="table"
      aria-label={t`Calendar by year and month`}
    >
      <span className="cal__y" />
      {heads.map((month, i) => (
        <span className="cal__h" key={`${month}-${i}`}>
          {month}
        </span>
      ))}
      <span className="cal__h">
        <Trans>Year</Trans>
      </span>

      {years.map((year) => (
        <Row
          key={year}
          year={year}
          byKey={byKey}
          peak={peak}
          total={totalOf.get(year)}
          full={full}
          selected={selected}
          onPick={onPick}
        />
      ))}

      {footer && <Footer cells={footer} total={footerTotal} />}
    </div>
  );
}

/** Same month across all years, so a quarterly payer shows up as one column. */
function Footer({ cells, total }: { cells: Array<{ month: number; text: string }>; total?: string }) {
  const textOf = new Map(cells.map((c) => [c.month, c.text]));
  return (
    <>
      <span className="cal__y">
        <Trans>all</Trans>
      </span>
      {Array.from({ length: 12 }, (_, i) => i + 1).map((month) => (
        <span className="cal__f num" key={month}>
          {textOf.get(month) ?? ""}
        </span>
      ))}
      <span className="cal__f num">{total ?? ""}</span>
    </>
  );
}

function Row({
  year,
  byKey,
  peak,
  total,
  full,
  selected,
  onPick,
}: {
  year: number;
  byKey: Map<string, CalendarCell>;
  peak: number;
  total?: string;
  full?: boolean;
  selected?: { year: number; month: number };
  onPick?: (year: number, month: number) => void;
}) {
  const { t } = useLingui();
  return (
    <>
      <span className="cal__y">{full ? `’${String(year).slice(2)}` : year}</span>
      {Array.from({ length: 12 }, (_, i) => i + 1).map((month) => {
        const cell = byKey.get(`${year}-${month}`);
        const title = cell?.title ?? t`${formatMonth(year, month)}: no data`;
        const style = cell ? { background: shade(cell.value, peak) } : undefined;
        const body = cell?.text && <span className="cal__v">{cell.text}</span>;

        if (!onPick) {
          return (
            <span key={month} className="cal__c" style={style} title={title}>
              {body}
            </span>
          );
        }
        return (
          <button
            key={month}
            type="button"
            className="cal__c"
            style={style}
            title={title}
            aria-pressed={selected?.year === year && selected.month === month}
            onClick={() => onPick(year, month)}
          >
            {body}
          </button>
        );
      })}
      <span className="cal__t">{total ?? ""}</span>
    </>
  );
}

/** Maps signed magnitude to the shared positive/negative palette. */
function shade(value: number, peak: number): string {
  const share = Math.round((Math.abs(value) / peak) * 72) + 8;
  const tone = value >= 0 ? "--pos" : "--neg";
  return `color-mix(in srgb, var(${tone}) ${share}%, var(--surface-3))`;
}
