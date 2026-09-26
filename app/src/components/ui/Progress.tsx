import type { ReactNode } from "react";
import { Bar, type BarSpan } from "./Bar";
import { trackWidth } from "../../lib/plot";
import { Figure, type Tone } from "./Figure";

export interface ProgressProps {
  /**
   * The headline figure: how far along, or how much of it there is. Absent where the figure is
   * already beside the block — a card whose value and target are its own row — and the track
   * plus the facts under it are all that is left to draw.
   */
  value?: ReactNode;
  tone?: Tone;
  delta?: ReactNode;
  deltaTone?: Tone;
  /** How far along, as a 0–1 share; clamped by `trackWidth`. */
  share: string | number | null | undefined;
  /** Colours the filled part; an allowance already spent is not a positive number. */
  barTone?: "pos" | "neg";
  /** The part beyond the mark, drawn hatched: over target, over allowance. */
  over?: BarSpan;
  /** Track height. `sm` for a block inside a card, where the figure is the row's own. */
  size?: "sm" | "md" | "lg";
  /** The two facts that read the track: what it is out of, and by when. */
  legend?: { left: ReactNode; right?: ReactNode };
  /** A line under the whole block — the thing to do about it. */
  note?: ReactNode;
}

/**
 * A figure over a track: a savings goal, a contribution limit, a target reached, a plan's pace.
 *
 * These shipped as four different layouts saying the same thing — a tile, a goal card, a limit
 * card, a rebalance row. One shape means a fifth costs a declaration rather than a stylesheet,
 * and the narrow-box behaviour is inherited instead of re-invented.
 */
export function Progress({
  value,
  tone,
  delta,
  deltaTone,
  share,
  barTone,
  over,
  size = "md",
  legend,
  note,
}: ProgressProps) {
  return (
    <div className="prog">
      {value !== undefined && <Figure value={value} tone={tone} delta={delta} deltaTone={deltaTone} />}
      <Bar size={size} fill={trackWidth(share)} tone={barTone} over={over} />
      {legend && (
        <div className="prog__legend">
          <span>{legend.left}</span>
          {legend.right !== undefined && <span>{legend.right}</span>}
        </div>
      )}
      {note !== undefined && note !== null && note !== "" && <div className="prog__note">{note}</div>}
    </div>
  );
}
