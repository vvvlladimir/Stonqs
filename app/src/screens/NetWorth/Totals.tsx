import { Trans, useLingui } from "@lingui/react/macro";
import { Metric, Metrics, Money, Percent } from "../../components/ui";
import type { NetWorth } from "../../lib/types";

/** The shelf: the two totals that must never be confused, and what they are made of. */
export function Totals({ reading }: { reading: NetWorth }) {
  const { t } = useLingui();
  const currency = reading.base_currency;

  return (
    <Metrics>
      <Metric
        label={t`Net worth`}
        value={<Money value={reading.net_base} currency={currency} />}
        hint={t`everything owned, minus everything owed`}
        tip={t`A balance sheet, not a performance: nothing here has a return.`}
      />
      <Metric
        label={t`Invested`}
        value={<Money value={reading.investments_base} currency={currency} />}
        hint={
          reading.invested_share !== null ? (
            <Trans>
              <Percent value={reading.invested_share} digits={0} /> of net worth
            </Trans>
          ) : undefined
        }
        tip={t`The portfolio's own value — the figure every other screen measures.`}
      />
      <Metric label={t`Other assets`} value={<Money value={reading.owned_base} currency={currency} />} />
      <Metric
        label={t`Debts`}
        value={<Money value={reading.owed_base} currency={currency} />}
        hint={
          reading.debt_to_assets !== null ? (
            <Trans>
              <Percent value={reading.debt_to_assets} digits={0} /> of everything owned
            </Trans>
          ) : undefined
        }
        tone={Number(reading.owed_base) > 0 ? "negative" : "neutral"}
      />
    </Metrics>
  );
}
