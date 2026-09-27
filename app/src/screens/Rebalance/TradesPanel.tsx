import type { UseQueryResult } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { CalendarDotsIcon } from "@phosphor-icons/react";
import { Async, Empty, Panel } from "../../components/ui";
import type {
  RebalanceItem,
  RebalancePlan,
  RebalanceTrade,
  SecurityRow,
  TaxonomyData,
} from "../../lib/types";
import { PlanTable } from "./PlanTable";
import type { CashRow } from "./model";

/** What to trade, with the offer to turn the buys into a standing plan. */
export function TradesPanel({
  plan,
  trades,
  cashRows,
  currency,
  taxonomy,
  securities,
  canPlan,
  onMakePlan,
}: {
  plan: UseQueryResult<RebalancePlan>;
  trades: { trade: RebalanceTrade; item: RebalanceItem }[];
  cashRows: CashRow[];
  currency: string;
  taxonomy: TaxonomyData | undefined;
  securities: SecurityRow[] | undefined;
  /** Set when there are buys; `null` when there are none, so no button is shown. */
  canPlan: boolean | null;
  onMakePlan: () => void;
}) {
  const { t } = useLingui();
  return (
    <Panel
      title={t`Trades`}
      table
      info={t`Quantities are rounded to each instrument's tradable step, such as whole shares.`}
      tools={
        canPlan !== null && (
          <button
            className="btn btn--sm"
            onClick={onMakePlan}
            disabled={!canPlan}
            title={canPlan ? undefined : t`A plan buys instruments, so it needs a securities account`}
          >
            <CalendarDotsIcon /> <Trans>Make a plan</Trans>
          </button>
        )
      }
    >
      <Async
        query={plan}
        isEmpty={() => trades.length === 0 && cashRows.length === 0}
        empty={
          <Empty title={t`Nothing to trade`}>
            <Trans>
              Every category sits within a step of its target, or the drift is smaller than one tradable unit.
            </Trans>
          </Empty>
        }
      >
        {() => (
          <PlanTable
            trades={trades}
            cashRows={cashRows}
            currency={currency}
            taxonomy={taxonomy}
            securities={securities}
          />
        )}
      </Async>
    </Panel>
  );
}
