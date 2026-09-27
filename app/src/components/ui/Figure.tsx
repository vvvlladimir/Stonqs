import type { ReactNode } from "react";

/** The tone a figure or a chip wears. Plain is the default and means "no verdict". */
export type Tone = "pos" | "neg" | "warn";

export interface FigureProps {
  /** The number itself, already formatted — `Money`, `Percent`, `Rate`, `Stat`. */
  value: ReactNode;
  /** Colours the number. A metric that is only ever a balance leaves this alone. */
  tone?: Tone;
  /** The change beside the value; rendered as a chip, never as coloured text. */
  delta?: ReactNode;
  deltaTone?: Tone;
  /** One line under the number saying what it is. It is what gives way in a short box. */
  note?: ReactNode;
  /** Size from the heading scale rather than the box; for screens, not tiles. */
  fixed?: boolean;
}

/** A number with an optional change and note, sized from its box. */
export function Figure({ value, tone, delta, deltaTone, note, fixed }: FigureProps) {
  return (
    <div className={`fig${fixed ? " fig--fixed" : ""}`}>
      <div className="fig__v">
        <span className={`fig__n num${tone ? ` ${tone}` : ""}`}>{value}</span>
        {delta !== undefined && delta !== null && <Delta tone={deltaTone}>{delta}</Delta>}
      </div>
      {note !== undefined && note !== null && note !== "" && <div className="fig__note">{note}</div>}
    </div>
  );
}

/** A small tinted chip for a change, a share or a state. Readable where a colour alone is not. */
export function Delta({ tone, children }: { tone?: Tone; children: ReactNode }) {
  return <span className={`delta num${tone ? ` ${tone}` : ""}`}>{children}</span>;
}
