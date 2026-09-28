import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { Metric, Metrics, Money, Percent } from "../../components/ui";
import { formatMoney, formatPercent, signOf } from "../../lib/format";
import type { IncomeData } from "../../lib/types";
import { ratio } from "./model";

/** The window's income: received, withheld, accrued, and what it yields on today's value. */
export function IncomeMetrics({
  data,
  currency,
  marketValue,
}: {
  data: IncomeData | undefined;
  currency: string;
  marketValue: string | undefined;
}) {
  const { t } = useLingui();
  return (
    <Metrics>
      <Metric
        label={t`Received in the period`}
        value={data ? <Money value={data.total.net_base} currency={currency} /> : "…"}
        hint={
          data
            ? t`${formatMoney(data.change_base, currency, { signed: true, compact: true })} versus the previous window`
            : undefined
        }
        tone={data ? signOf(data.change_base) : "neutral"}
        tip={t`What reached the cash accounts, net of withholding tax.`}
      />
      <Metric
        label={t`Tax withheld`}
        value={data ? <Money value={data.total.taxes_base} currency={currency} /> : "…"}
        hint={
          data
            ? t`${formatPercent(ratio(data.total.taxes_base, data.total.gross_base))} of the accrued amount`
            : undefined
        }
        tip={t`Withheld at the source, so it is not in what was received.`}
      />
      <Metric
        label={t`Accrued`}
        value={data ? <Money value={data.total.gross_base} currency={currency} /> : "…"}
        hint={data ? plural(data.total.events, { one: "# payment", other: "# payments" }) : undefined}
      />
      <Metric
        label={t`Portfolio yield`}
        value={
          data && marketValue ? <Percent value={ratio(data.total.gross_base, marketValue)} digits={1} /> : "…"
        }
        hint={
          marketValue
            ? t`against a value of ${formatMoney(marketValue, currency, { compact: true })}`
            : undefined
        }
        tip={t`Income in the window against current market value.`}
      />
    </Metrics>
  );
}
