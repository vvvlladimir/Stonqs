import type { CSSProperties } from "react";
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

  // Every cell states its month and its year, and the stylesheet places it from them: the same
  // markup then reads months-across in a wide box and months-down in a narrow one, where twelve
  // columns would be twelve smudges. See `styles/ui/calendar.css`.
  return (
    <div className={`calbox${height === "fill" ? " cal--fillbox" : ""}`}>
      <div
        className={`cal${full ? " cal--full" : ""}${height === "fill" ? " cal--fill" : ""}`}
        style={{ "--years": years.length } as CSSProperties}
        role="table"
        aria-label={t`Calendar by year and month`}
      >
        <span className="cal__y cal__corner" />
        {heads.map((month, i) => (
          <span className="cal__h" key={`${month}-${i}`} style={{ "--m": i } as CSSProperties}>
            {month}
          </span>
        ))}
        <span className="cal__h cal__h--total">
          <Trans>Year</Trans>
        </span>

        {years.map((year, index) => (
          <Row
            key={year}
            year={year}
            index={index}
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
    </div>
  );
}

/** Same month across all years, so a quarterly payer shows up as one column. */
function Footer({ cells, total }: { cells: Array<{ month: number; text: string }>; total?: string }) {
  const textOf = new Map(cells.map((c) => [c.month, c.text]));
  return (
    <>
      <span className="cal__f cal__f--head">
        <Trans>all</Trans>
      </span>
      {MONTH_NUMBERS.map((month) => (
        <span className="cal__f num" key={month} style={{ "--m": month - 1 } as CSSProperties}>
          {textOf.get(month) ?? ""}
        </span>
      ))}
      <span className="cal__f cal__f--total num">{total ?? ""}</span>
    </>
  );
}

function Row({
  year,
  index,
  byKey,
  peak,
  total,
  full,
  selected,
  onPick,
}: {
  year: number;
  /** Which row of the grid this year is; the stylesheet turns it into a row or a column. */
  index: number;
  byKey: Map<string, CalendarCell>;
  peak: number;
  total?: string;
  full?: boolean;
  selected?: { year: number; month: number };
  onPick?: (year: number, month: number) => void;
}) {
  const { t } = useLingui();
  const at = { "--y": index } as CSSProperties;
  return (
    <>
      <span className="cal__y" style={at}>
        {full ? `’${String(year).slice(2)}` : year}
      </span>
      {MONTH_NUMBERS.map((month) => {
        const cell = byKey.get(`${year}-${month}`);
        const title = cell?.title ?? t`${formatMonth(year, month)}: no data`;
        const style = {
          ...at,
          "--m": month - 1,
          ...(cell ? { background: shade(cell.value, peak) } : {}),
        } as CSSProperties;
        const body = cell?.text && <span className="cal__v">{cell.text}</span>;

        if (!onPick) {
          return (
            <span key={month} className="cal__c" style={style} data-tip={title}>
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
            data-tip={title}
            aria-pressed={selected?.year === year && selected.month === month}
            onClick={() => onPick(year, month)}
          >
            {body}
          </button>
        );
      })}
      <span className="cal__t" style={at}>
        {total ?? ""}
      </span>
    </>
  );
}

/** Maps signed magnitude to the shared positive/negative palette. */
function shade(value: number, peak: number): string {
  const share = Math.round((Math.abs(value) / peak) * 72) + 8;
  const tone = value >= 0 ? "--pos" : "--neg";
  return `color-mix(in srgb, var(${tone}) ${share}%, var(--surface-3))`;
}
