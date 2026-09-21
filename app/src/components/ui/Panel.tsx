import type { ReactNode } from "react";
import { InfoHeading } from "./InfoTip";

/** Shared surface for dashboard panels and table sections. */
interface Props {
  title?: ReactNode;
  /** Optional controls aligned to the right of the header. */
  tools?: ReactNode;
  /** A figure beside the title — a count, a window, a name. An explanation goes to `info`. */
  note?: ReactNode;
  /** One sentence on what the panel shows, behind an info mark beside the title. */
  info?: string;
  table?: boolean;
  /** A plot lives here: the body is a tile of fixed height and the chart is told to fill it. */
  chart?: boolean;
  className?: string;
  children: ReactNode;
}

export function Panel({ title, tools, note, info, table, chart, className, children }: Props) {
  const classes = ["panel", table ? "panel--table" : "", chart ? "panel--chart" : "", className ?? ""]
    .filter(Boolean)
    .join(" ");
  return (
    <section className={classes}>
      {(title || tools || note) && (
        <div className="panel__head">
          {title && <InfoHeading title={title} info={info} />}
          {note && <span className="panel__note">{note}</span>}
          {tools && <div className="panel__tools">{tools}</div>}
        </div>
      )}
      <div className="panel__body">{children}</div>
    </section>
  );
}
