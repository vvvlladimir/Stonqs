import { Children, useState, type ReactNode } from "react";
import { Plural, Trans } from "@lingui/react/macro";
import { CaretDownIcon } from "@phosphor-icons/react";
import { useIsWide } from "../../lib/useLayout";

/** Tiles a page shelf fits on one row, wide and narrow. */
const PER_ROW = { wide: 4, narrow: 2 };

/** Metric tiles with optional paired layout and semantic result coloring. */
export function Metrics({
  children,
  pair,
  inset,
}: {
  children: ReactNode;
  pair?: boolean;
  /** Tiles of a panel rather than the page's shelf: no frame of their own, only dividers. */
  inset?: boolean;
}) {
  const isWide = useIsWide();
  const [open, setOpen] = useState(false);

  const classes = ["metrics"];
  if (pair) classes.push("metrics--pair");
  if (inset) classes.push("metrics--inset");
  const shelf = !pair && !inset;

  // The shelf is one row of figures; the rest are kept, not shown. A panel's tiles are part of
  // its answer and are never folded away.
  const tiles = Children.toArray(children);
  const perRow = isWide ? PER_ROW.wide : PER_ROW.narrow;
  const folds = shelf && tiles.length > perRow;
  const hidden = tiles.length - perRow;

  return (
    <div className={classes.join(" ")} data-slot={shelf ? "metrics" : undefined}>
      {folds && !open ? tiles.slice(0, perRow) : tiles}
      {folds && (
        <button type="button" className="metricfold" aria-expanded={open} onClick={() => setOpen(!open)}>
          {open ? (
            <Trans>Show less</Trans>
          ) : (
            <Plural value={hidden} one="# more figure" other="# more figures" />
          )}
          <CaretDownIcon aria-hidden />
        </button>
      )}
    </div>
  );
}

interface MetricProps {
  label: ReactNode;
  /** Preformatted value supplied by the caller. */
  value: ReactNode;
  hint?: ReactNode;
  tone?: "positive" | "negative" | "zero" | "neutral";
  tip?: string;
}

export function Metric({ label, value, hint, tone = "neutral", tip }: MetricProps) {
  const sign = tone === "positive" ? " pos" : tone === "negative" ? " neg" : "";
  return (
    <div className="metric" data-tip={tip}>
      <div className="metric__k">{label}</div>
      <div className={`metric__v num${sign}`}>{value}</div>
      {hint && <div className="metric__h">{hint}</div>}
    </div>
  );
}

/**
 * Label-and-figure pairs: a definition list, not a shelf of tiles. Where a `Metric` gives one
 * number room, this gives five of them a column each — the widest label sets the left column and
 * every figure lines up on the right, which is what makes a set of statistics scannable.
 */
export function Facts({ children }: { children: ReactNode }) {
  return <dl className="facts">{children}</dl>;
}

/** One pair inside `Facts`; a fragment, so the pairs share the list's own grid. */
export function Fact({ label, value }: { label: ReactNode; value: ReactNode }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </>
  );
}
