import { useLingui } from "@lingui/react/macro";
import { Metric, Metrics, Money } from "../../components/ui";
import { formatMoney } from "../../lib/format";
import type { RebalancePlan } from "../../lib/types";
import { formatPoints, largestDrift } from "./model";

/** The plan in four figures: its base, the worst drift, what it buys and sells, what is left. */
export function RebalanceMetrics({
  plan,
  currency,
  withCash,
  tradeCount,
}: {
  plan: RebalancePlan | undefined;
  currency: string;
  withCash: boolean;
  tradeCount: number;
}) {
  const { t, i18n } = useLingui();
  const worst = largestDrift(plan?.items ?? []);
  return (
    <Metrics>
      <Metric
        label={t`Portfolio`}
        value={plan ? <Money value={plan.total_base} currency={currency} digits={0} /> : "…"}
        hint={withCash ? t`including the new money` : t`the targets are derived from it`}
      />
      <Metric
        label={t`Largest drift`}
        value={worst ? formatPoints(i18n, worst.points) : "—"}
        tone={worst && worst.points > 0 ? "negative" : "neutral"}
        hint={
          worst
            ? worst.points > 0
              ? t`${worst.item.label}, above target`
              : t`${worst.item.label}, below target`
            : t`the plan has no targets`
        }
      />
      <Metric
        label={t`To buy`}
        value={plan ? <Money value={plan.cash_used_base} currency={currency} digits={0} /> : "…"}
        hint={
          plan ? t`to sell ${formatMoney(plan.sell_base, currency, { digits: 0 })}` : t`and how much to sell`
        }
        tip={t`Buys and sells apart: a sale realizes a result and can be taxable.`}
      />
      <Metric
        label={t`Trades`}
        value={plan ? String(tradeCount) : "…"}
        hint={
          plan
            ? t`${formatMoney(plan.cash_left_base, currency, { digits: 0 })} left over`
            : t`after rounding to the step`
        }
        tip={t`Quantities round down to the tradable step, so some cash stays cash.`}
      />
    </Metrics>
  );
}
