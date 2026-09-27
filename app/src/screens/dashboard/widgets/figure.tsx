import type { ReactNode } from "react";
import { useLingui } from "@lingui/react/macro";
import { Figure as UiFigure, type Tone } from "../../../components/ui";
import { periodLabel } from "../../../lib/periods";
import { formatDay } from "../../../lib/format";
import { type MetricCtx, useRange } from "./model";

/** `foot` is the metric's own note; the widget's setting decides whether it shows. */
export function Figure({
  ctx,
  value,
  delta,
  tone,
  deltaTone,
  foot,
}: {
  ctx: MetricCtx;
  value: ReactNode;
  delta?: ReactNode;
  tone?: Tone;
  deltaTone?: Tone;
  foot?: ReactNode;
}) {
  // What the line under the number says is this file's business; how a figure looks is not.
  return <UiFigure value={value} tone={tone} delta={delta} deltaTone={deltaTone} note={useFoot(ctx, foot)} />;
}

/** What the line under the value says: the metric's own note unless the widget names another. */
function useFoot(ctx: MetricCtx, own: ReactNode): ReactNode {
  const { i18n } = useLingui();
  const range = useRange(ctx.date, ctx.period);
  switch (ctx.foot) {
    case "none":
      return null;
    case "dates":
      return range ? `${formatDay(range.from)} — ${formatDay(range.to)}` : null;
    case "period":
      return range ? periodLabel(i18n, range) : null;
    default:
      return own;
  }
}
