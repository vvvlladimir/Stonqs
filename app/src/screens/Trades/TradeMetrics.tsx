import { useLingui } from "@lingui/react/macro";
import { Metric, Metrics as MetricStrip, Money, Num, Percent } from "../../components/ui";
import { signOf } from "../../lib/format";
import type { TradesData } from "../../lib/types";
import { days } from "./model";

/**
 * A trade ledger read twice: what leaving positions brought in, and what is still riding.
 * The two never merge into one average — an open trade has no exit to divide by.
 */
export function TradeMetrics({ data }: { data?: TradesData }) {
  const { t } = useLingui();
  const currency = data?.base_currency ?? "";
  const closed = data?.closed_stats;
  const open = data?.open_stats;

  return (
    <MetricStrip>
      <Metric
        label={t`Closed in the period`}
        value={closed ? <Num>{closed.trades}</Num> : "…"}
        hint={
          closed ? (
            <>
              <Num>{closed.winners}</Num> {t`up`} · <Num>{closed.losers}</Num> {t`down`}
            </>
          ) : undefined
        }
        tip={t`A trade closes when the last share of one purchase is sold.`}
      />
      <Metric
        label={t`Hit rate`}
        value={closed?.win_rate ? <Percent value={closed.win_rate} digits={0} /> : "—"}
        hint={t`of the trades closed here`}
        tip={t`Share of closed trades that ended above their entry value.`}
      />
      <Metric
        label={t`Result of closed trades`}
        value={closed ? <Money value={closed.pnl_base} currency={currency} signed /> : "…"}
        tone={closed ? signOf(closed.pnl_base) : "neutral"}
        hint={
          closed ? (
            <>
              {t`entry`} <Money value={closed.entry_value_base} currency={currency} />
            </>
          ) : undefined
        }
        tip={t`Exit value minus entry value on closed trades, commissions included.`}
      />
      <Metric
        label={t`Average holding`}
        value={closed && closed.trades > 0 ? days(closed.average_holding_days) : "—"}
        hint={t`weighted by what was invested`}
        tip={t`How long closed trades lived, weighted by the money at work.`}
      />
      <Metric
        label={t`Still open`}
        value={open ? <Num>{open.trades}</Num> : "…"}
        hint={
          open && open.trades > 0 ? (
            <>
              {days(open.average_holding_days)} {t`on average`}
            </>
          ) : undefined
        }
        tip={t`Purchases still held at the end of the period.`}
      />
      <Metric
        label={t`Result of open trades`}
        value={open ? <Money value={open.pnl_base} currency={currency} signed /> : "…"}
        tone={open ? signOf(open.pnl_base) : "neutral"}
        hint={
          open ? (
            <>
              {t`worth`} <Money value={open.exit_value_base} currency={currency} />
            </>
          ) : undefined
        }
        tip={t`What open trades would make if closed at the last price.`}
      />
      <Metric
        label={t`Turnover`}
        value={data?.turnover_rate ? <Percent value={data.turnover_rate} digits={1} /> : "—"}
        hint={
          data ? (
            <>
              <Money value={data.volume.bought_base} currency={currency} /> {t`bought`} ·{" "}
              <Money value={data.volume.sold_base} currency={currency} /> {t`sold`}
            </>
          ) : undefined
        }
        tip={t`Everything bought and sold in the period against the capital at work.`}
      />
    </MetricStrip>
  );
}
