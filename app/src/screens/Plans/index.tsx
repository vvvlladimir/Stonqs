import { plural } from "@lingui/core/macro";
import { Command } from "../../lib/commands";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { CalendarBlankIcon, PencilSimpleIcon, PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { api, today } from "../../lib/api";
import { formatDay, formatMoney } from "../../lib/format";
import { planCadenceLabel } from "../../lib/kinds";
import { EMPTY_PLAN, planToInput } from "../../lib/plans";
import { affects, useAccounts, useInvalidate, usePlans, useSecurities } from "../../lib/queries";
import { Page } from "../../components/Page";
import { PlanForm } from "../../components/domain/PlanForm";
import { SecurityLink } from "../../components/domain/SecurityCardProvider";
import {
  Badge,
  Banner,
  Buttons,
  Empty,
  ErrorText,
  List,
  ListRow,
  Money,
  Pending,
  QueryError,
  ShareBar,
} from "../../components/ui";
import type { PlanInput, PlanRow } from "../../lib/types";
import { DuePanel } from "./DuePanel";
import { GoalsPanel } from "./Goals";

export function Plans() {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const plans = usePlans();
  const accounts = useAccounts();
  const securities = useSecurities();
  const [draft, setDraft] = useState<PlanInput | null>(null);
  const [due, setDue] = useState<PlanRow | null>(null);

  const save = useMutation({
    mutationFn: api.planSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.plans);
    },
  });

  const remove = useMutation({
    mutationFn: api.planDelete,
    onSuccess: () => invalidate(...affects.plans),
  });

  if (plans.isError) return <QueryError error={plans.error} />;
  if (!plans.data || !accounts.data || !securities.data) return <Pending />;

  const { rows, base_currency, monthly_base } = plans.data;
  const accountRows = accounts.data;
  const securityRows = securities.data;
  const owed = rows.filter((row) => row.due_count > 0);
  const owedTotal = owed.reduce((sum, row) => sum + row.due_count, 0);

  const newPlan = () =>
    setDraft({
      ...EMPTY_PLAN,
      currency: base_currency,
      account_id: accountRows[0]?.id ?? "",
      start: today(),
    });

  const del = (row: PlanRow) => {
    if (confirm(t`Delete the plan "${row.plan.name}"? Recorded transactions stay where they are.`)) {
      remove.mutate(row.plan.id);
    }
  };

  return (
    <Page
      archetype="registry"
      title={t`Plans`}
      summary={[
        plural(rows.length, { one: "# plan", other: "# plans" }),
        t`${formatMoney(monthly_base, base_currency)} a month`,
      ].join(" · ")}
      actions={
        <>
          <button className="btn" onClick={newPlan} disabled={accountRows.length === 0}>
            <PlusIcon /> <Trans>New plan</Trans>
          </button>
          <Command id="new" label={t`New plan`} run={newPlan} disabled={accountRows.length === 0} />
          <Command id="newPlan" run={newPlan} disabled={accountRows.length === 0} />
        </>
      }
      banner={
        owedTotal > 0 ? (
          <Banner
            tone="info"
            action={
              <button className="btn btn--sm" onClick={() => setDue(owed[0])}>
                <Trans>Review</Trans>
              </button>
            }
          >
            {plural(owedTotal, {
              one: "# contribution has come due and is not recorded yet",
              other: "# contributions have come due and are not recorded yet",
            })}
          </Banner>
        ) : undefined
      }
    >
      <ErrorText error={remove.error} />

      {draft && (
        <PlanForm
          draft={draft}
          accounts={accountRows}
          securities={securityRows}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onCancel={() => setDraft(null)}
          pending={save.isPending}
          error={save.error}
        />
      )}

      {due && <DuePanel row={due} onClose={() => setDue(null)} />}

      {rows.length === 0 ? (
        <Empty
          title={t`No plans yet`}
          action={
            <button className="btn" onClick={newPlan} disabled={accountRows.length === 0}>
              <PlusIcon /> <Trans>New plan</Trans>
            </button>
          }
        >
          <Trans>
            A plan is a contribution on a schedule: how much, how often, and what it buys. It proposes the
            transactions and you confirm them, so nothing is written behind your back.
          </Trans>
        </Empty>
      ) : (
        <List variant="cards">
          {rows.map((row) => (
            <PlanCard
              key={row.plan.id}
              row={row}
              cadence={planCadenceLabel(i18n, row.plan.schedule.unit, row.plan.schedule.count)}
              onDue={() => setDue(row)}
              onEdit={() => setDraft(planToInput(row))}
              onDelete={() => del(row)}
            />
          ))}
        </List>
      )}

      <GoalsPanel baseCurrency={base_currency} />
    </Page>
  );
}

/**
 * One plan as a card: what it pays, when it pays next, and — the part a row of tickers cannot
 * show — how the money is divided, as one track with its legend underneath.
 */
function PlanCard({
  row,
  cadence,
  onDue,
  onEdit,
  onDelete,
}: {
  row: PlanRow;
  cadence: string;
  onDue: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t } = useLingui();
  const { plan } = row;

  const split =
    row.legs.length === 0 ? (
      <span className="dim">
        <Trans>Stays as cash on the account</Trans>
      </span>
    ) : (
      <span className="stack">
        <ShareBar
          label={t`How the contribution is split`}
          slices={row.legs.map((leg) => ({
            key: leg.security_id,
            label: <SecurityLink id={leg.security_id}>{leg.symbol}</SecurityLink>,
            name: leg.symbol,
            share: leg.share,
          }))}
        />
      </span>
    );

  return (
    <ListRow
      box
      top
      title={plan.name}
      sub={
        <>
          {cadence} · {row.account_name}
          {!plan.active && (
            <>
              {" "}
              <Badge tone="warn">
                <Trans>Stopped</Trans>
              </Badge>
            </>
          )}
        </>
      }
      value={<Money value={plan.amount} currency={plan.currency} />}
      meta={row.next_date ? <Trans>next {formatDay(row.next_date)}</Trans> : <Trans>finished</Trans>}
      // Edit and delete stand in `end`, not in `actions`: row actions are revealed on hover and
      // hold their width while invisible, which on a card leaves a hole beside the amount.
      end={
        <Buttons>
          {row.due_count > 0 && (
            <button className="btn btn--sm" onClick={onDue} title={t`Review and record`}>
              <CalendarBlankIcon /> {plural(row.due_count, { one: "# due", other: "# due" })}
            </button>
          )}
          <button className="iconbtn iconbtn--sm" onClick={onEdit} title={t`Edit`} aria-label={t`Edit`}>
            <PencilSimpleIcon />
          </button>
          <button
            className="iconbtn iconbtn--sm iconbtn--danger"
            onClick={onDelete}
            title={t`Delete`}
            aria-label={t`Delete`}
          >
            <TrashIcon />
          </button>
        </Buttons>
      }
      foot={split}
    />
  );
}
