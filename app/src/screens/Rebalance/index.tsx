import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { CalendarDotsIcon, PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { DriftBars } from "../../components/charts";
import { Page } from "../../components/Page";
import {
  Async,
  Choice,
  Empty,
  Metric,
  Metrics,
  Money,
  Panel,
  Pending,
  QueryError,
  Seg,
} from "../../components/ui";
import { formatDay, formatMoney } from "../../lib/format";
import { planFromTrades } from "../../lib/plans";
import { slotOfNode } from "../../lib/taxonomy";
import {
  affects,
  useAccounts,
  useDashboard,
  useInvalidate,
  usePortfolio,
  useRebalance,
  useSecurities,
  useTargets,
  useTaxonomies,
} from "../../lib/queries";
import { PlanForm } from "../../components/domain/PlanForm";
import type { PlanInput } from "../../lib/types";
import { CashPanel } from "./CashPanel";
import { PlanTable } from "./PlanTable";
import { TargetForm } from "./TargetForm";
import { cashInPlan, formatPoints, largestDrift } from "./model";
import { useAsOf } from "../../lib/asOf";

export function Rebalance() {
  const { t, i18n } = useLingui();
  const date = useAsOf().date;
  const invalidate = useInvalidate();
  const [targetId, setTargetId] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [cash, setCash] = useState("");
  const [mode, setMode] = useState<"both" | "buy">("both");
  const [planDraft, setPlanDraft] = useState<PlanInput | null>(null);

  const targets = useTargets();
  const taxonomies = useTaxonomies();
  const portfolio = usePortfolio();
  const securities = useSecurities();
  const accounts = useAccounts();
  // Cash is part of target denominators even though it never creates a trade.
  const dashboard = useDashboard(date);
  const selected = targetId ?? targets.data?.[0]?.id ?? null;

  const plan = useRebalance(selected, date, cash.trim() || null, mode === "both");

  const savePlan = useMutation({
    mutationFn: api.planSave,
    onSuccess: () => {
      setPlanDraft(null);
      invalidate(...affects.plans);
    },
  });

  const remove = useMutation({
    mutationFn: api.targetDelete,
    onSuccess: () => {
      setTargetId(null);
      invalidate(...affects.targets);
    },
  });

  if (targets.isError) return <QueryError error={targets.error} />;
  if (targets.isPending || taxonomies.isPending) return <Pending />;

  const currency = portfolio.data?.base_currency ?? "";
  const target = targets.data.find((t) => t.id === selected) ?? null;
  const taxonomy = taxonomies.data?.find((t) => t.id === target?.taxonomy_id);
  const items = plan.data?.items ?? [];
  const trades = items.flatMap((item) => item.trades.map((trade) => ({ trade, item })));
  const worst = largestDrift(items);
  const cashRows = cashInPlan(items, taxonomy, dashboard.data?.cash, dashboard.data?.accounts, currency);
  const buys = trades.map(({ trade }) => trade).filter((trade) => !trade.quantity.startsWith("-"));
  const depot = accounts.data?.find((a) => a.kind === "SECURITIES");

  /**
   * The drift turned into a standing order: what the plan says to buy today becomes the split of
   * every future contribution, so the same decision is made once instead of every month. Sales are
   * dropped — a contribution pays money in — and the amount defaults to the new money being
   * allocated, which is the figure the screen was just asked about.
   */
  const makePlan = () =>
    setPlanDraft(
      planFromTrades({
        name: target?.name ?? t`Rebalance`,
        accountId: depot?.id ?? "",
        currency,
        amount: cash.trim() || (plan.data?.cash_used_base ?? ""),
        trades: buys,
      }),
    );

  return (
    <Page
      archetype="analysis"
      title={t`Rebalance`}
      asOf={formatDay(date)}
      note={target?.name}
      controls={
        <>
          {targets.data.length > 1 && (
            <Choice
              wide
              label={t`Target`}
              value={selected ?? ""}
              onChange={setTargetId}
              options={targets.data.map((t) => ({ value: t.id, label: t.name }))}
            />
          )}
          <Seg
            label={t`Mode`}
            value={mode}
            onChange={setMode}
            options={[
              { value: "both", label: t`Buy and sell` },
              { value: "buy", label: t`Buy only` },
            ]}
          />
        </>
      }
      actions={
        <>
          <button className="btn" onClick={() => setEditing(true)}>
            <PlusIcon /> <Trans>New target</Trans>
          </button>
          {selected && (
            <button
              className="iconbtn iconbtn--danger"
              onClick={() => remove.mutate(selected)}
              title={t`Delete target`}
              aria-label={t`Delete target`}
            >
              <TrashIcon />
            </button>
          )}
        </>
      }
      metrics={
        <Metrics>
          <Metric
            label={t`Portfolio`}
            value={plan.data ? <Money value={plan.data.total_base} currency={currency} digits={0} /> : "…"}
            hint={cash.trim() ? t`including the new money` : t`the targets are derived from it`}
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
            value={
              plan.data ? <Money value={plan.data.cash_used_base} currency={currency} digits={0} /> : "…"
            }
            hint={
              plan.data
                ? t`to sell ${formatMoney(plan.data.sell_base, currency, { digits: 0 })}`
                : t`and how much to sell`
            }
            tip={t`Buys and sells apart: a sale realizes a result and can be taxable.`}
          />
          <Metric
            label={t`Trades`}
            value={plan.data ? String(trades.length) : "…"}
            hint={
              plan.data
                ? t`${formatMoney(plan.data.cash_left_base, currency, { digits: 0 })} left over`
                : t`after rounding to the step`
            }
            tip={t`Quantities round down to the tradable step, so some cash stays cash.`}
          />
        </Metrics>
      }
      banner={plan.isError ? <QueryError error={plan.error} /> : undefined}
    >
      {targets.data.length === 0 ? (
        <Empty
          title={t`No targets`}
          action={
            <button className="btn" onClick={() => setEditing(true)}>
              <PlusIcon /> <Trans>New target</Trans>
            </button>
          }
        >
          <Trans>
            A target is a set of weights over taxonomy nodes. Without one there is nothing to compare against:
            "5,000 short of target" and "5,000 sitting in cash" are different news.
          </Trans>
        </Empty>
      ) : (
        <>
          <CashPanel cash={cash} onCash={setCash} total={plan.data?.total_base} currency={currency} />

          <Panel
            title={t`Deviation from target`}
            info={t`The fill is the current weight and the tick the target; hatching marks an overweight, a dashed run a shortfall.`}
          >
            <Async query={plan}>
              {() => (
                <DriftBars
                  items={items}
                  currency={currency}
                  slotOf={(nodeId) => slotOfNode(taxonomy, nodeId)}
                />
              )}
            </Async>
          </Panel>

          <Panel
            title={t`Trades`}
            table
            info={t`Quantities are rounded to each instrument's tradable step, such as whole shares.`}
            tools={
              buys.length > 0 && (
                <button
                  className="btn btn--sm"
                  onClick={makePlan}
                  disabled={!depot}
                  title={depot ? undefined : t`A plan buys instruments, so it needs a securities account`}
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
                    Every category sits within a step of its target, or the drift is smaller than one tradable
                    unit.
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
                  securities={securities.data}
                />
              )}
            </Async>
          </Panel>
        </>
      )}

      {planDraft && (
        <PlanForm
          draft={planDraft}
          accounts={accounts.data ?? []}
          securities={securities.data ?? []}
          onChange={setPlanDraft}
          onSubmit={() => savePlan.mutate(planDraft)}
          onCancel={() => setPlanDraft(null)}
          pending={savePlan.isPending}
          error={savePlan.error}
          title={t`New plan from this rebalance`}
        />
      )}

      {editing && (
        <TargetForm
          taxonomies={taxonomies.data ?? []}
          onClose={() => setEditing(false)}
          onDone={() => {
            setEditing(false);
            invalidate(...affects.targets);
          }}
        />
      )}
    </Page>
  );
}
