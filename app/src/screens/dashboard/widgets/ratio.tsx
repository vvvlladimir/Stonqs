import { Trans, useLingui } from "@lingui/react/macro";
import { Async, Percent, QueryError, Stat } from "../../../components/ui";
import { useDashboard, useIncomeOver, usePerformance } from "../../../lib/queries";
import { formatMoney } from "../../../lib/format";
import type { MoneyString } from "../../../lib/types";
import {
  RATIO_TERMS,
  periodOf,
  ratioTerms,
  sourceOf,
  type RatioData,
  type WidgetProps,
  useRange,
} from "./model";

import { Figure } from "./figure";

/** One figure over another; a share of two displayed values is rendering, not money. */
export function RatioWidget({ widget, date, period }: WidgetProps) {
  const { i18n } = useLingui();
  const ctx = { date, period: periodOf(widget, period), source: sourceOf(widget) };
  const { top, bottom } = ratioTerms(widget.cfg);
  const range = useRange(ctx.date, ctx.period);
  const needsPeriod = RATIO_TERMS[top].periodic || RATIO_TERMS[bottom].periodic;

  const dashboard = useDashboard(ctx.date, ctx.source);
  const performance = usePerformance(needsPeriod ? range : undefined, ctx.source);
  const income = useIncomeOver(needsPeriod ? range : undefined, null, ctx.source);

  if (needsPeriod && !range)
    return (
      <p className="muted">
        <Trans>No transactions.</Trans>
      </p>
    );
  // The tile waits on `dashboard` below; a period query that failed would otherwise leave it
  // waiting for an answer that is not coming.
  const failed = needsPeriod ? (performance.error ?? income.error) : null;
  if (failed) return <QueryError error={failed} />;

  const data: RatioData = {
    valuation: dashboard.data?.valuation,
    summary: performance.data?.summary,
    costs: performance.data?.costs,
    income: income.data?.total,
  };
  const numerator = RATIO_TERMS[top].pick(data);
  const denominator = RATIO_TERMS[bottom].pick(data);
  const currency = dashboard.data?.valuation.base_currency ?? "";

  return (
    <Async query={dashboard}>
      {() => (
        <Figure
          ctx={ctx}
          value={<RatioValue top={numerator} bottom={denominator} multiple={widget.cfg.as === "multiple"} />}
          foot={
            numerator !== undefined && denominator !== undefined
              ? `${formatMoney(numerator, currency, { compact: true })} / ${formatMoney(denominator, currency, { compact: true })}`
              : i18n._(RATIO_TERMS[top].label)
          }
        />
      )}
    </Async>
  );
}

/** A ratio has no answer when the divisor is zero, and that is an absent figure, not a zero. */
function RatioValue({
  top,
  bottom,
  multiple,
}: {
  top?: MoneyString;
  bottom?: MoneyString;
  multiple?: boolean;
}) {
  if (top === undefined || bottom === undefined) return <>…</>;
  const divisor = Number(bottom);
  if (!Number.isFinite(divisor) || divisor === 0) return <>—</>;
  const ratio = Number(top) / divisor;
  if (multiple)
    return (
      <>
        <Stat value={ratio} digits={2} />
        <span className="cur">×</span>
      </>
    );
  return <Percent value={String(ratio)} digits={1} />;
}
