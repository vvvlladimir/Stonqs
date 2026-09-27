import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { useAsOf } from "../../lib/asOf";
import { affects, useAccounts, useGoals, useInvalidate } from "../../lib/queries";
import { Async, Empty, ErrorText, List, Panel } from "../../components/ui";
import type { GoalInput, GoalRow } from "../../lib/types";
import { GoalCard } from "./GoalCard";
import { GoalForm } from "./GoalForm";

const BLANK: GoalInput = {
  id: null,
  name: "",
  target_amount: "",
  currency: "",
  target_date: null,
  monthly_amount: null,
  expected_return: null,
  note: null,
  accounts: [],
};

/** Not scoped: a goal names its own accounts. */
export function GoalsPanel({ baseCurrency }: { baseCurrency: string }) {
  const { t } = useLingui();
  const date = useAsOf().date;
  const invalidate = useInvalidate();
  const goals = useGoals(date);
  const accounts = useAccounts();
  const [draft, setDraft] = useState<GoalInput | null>(null);

  const save = useMutation({
    mutationFn: api.goalSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.goals);
    },
  });
  const remove = useMutation({
    mutationFn: api.goalDelete,
    onSuccess: () => invalidate(...affects.goals),
  });

  const edit = (row: GoalRow) =>
    setDraft({
      id: row.goal.id,
      name: row.goal.name,
      target_amount: row.goal.target_amount,
      currency: row.goal.currency,
      target_date: row.goal.target_date,
      monthly_amount: row.goal.monthly_amount,
      // Stored as a fraction, typed as a percent.
      expected_return: String(Number(row.goal.expected_return) * 100),
      note: row.goal.note,
      accounts: row.goal.accounts,
    });

  const del = (row: GoalRow) => {
    if (confirm(t`Delete the goal "${row.goal.name}"? Nothing else changes.`)) {
      remove.mutate(row.goal.id);
    }
  };

  return (
    <Panel
      title={t`Goals`}
      info={t`An amount you mean to have by a date, measured against the accounts you name.`}
      tools={
        <button
          type="button"
          className="btn btn--sm"
          onClick={() => setDraft({ ...BLANK, currency: baseCurrency })}
        >
          <PlusIcon /> <Trans>New goal</Trans>
        </button>
      }
    >
      <ErrorText error={remove.error} />
      <Async
        query={goals}
        empty={
          <Empty title={t`No goals yet`}>
            <Trans>
              A goal is an amount by a date — a deposit, a car, a year of runway. It changes nothing in the
              portfolio; it says how far the accounts you pick are from that amount.
            </Trans>
          </Empty>
        }
      >
        {(rows) => (
          <List variant="cards">
            {rows.map((row) => (
              <GoalCard key={row.goal.id} row={row} onEdit={() => edit(row)} onDelete={() => del(row)} />
            ))}
          </List>
        )}
      </Async>

      {draft && (
        <GoalForm
          draft={draft}
          accounts={accounts.data ?? []}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onClose={() => setDraft(null)}
          busy={save.isPending}
          error={save.error}
        />
      )}
    </Panel>
  );
}
