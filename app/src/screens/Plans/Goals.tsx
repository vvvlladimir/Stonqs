import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PencilSimpleIcon, PlusIcon, TrashIcon } from "@phosphor-icons/react";

import { api } from "../../lib/api";
import { useAsOf } from "../../lib/asOf";
import { formatDay, formatPercent, toNumber } from "../../lib/format";
import { affects, useAccounts, useGoals, useInvalidate } from "../../lib/queries";
import {
  Async,
  Badge,
  Bar,
  CheckField,
  Empty,
  ErrorText,
  Field,
  FieldPair,
  FormDialog,
  List,
  ListRow,
  Money,
  Panel,
} from "../../components/ui";
import type { AccountRow, GoalInput, GoalRow } from "../../lib/types";

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

/**
 * Goals live beside the plans because they are the same conversation — what is being saved for.
 * They are not scoped: a goal names the accounts that count towards it, so the picker changes
 * nothing about it.
 */
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

/** One goal: how far along it is, and the one figure that answers "is this enough". */
function GoalCard({ row, onEdit, onDelete }: { row: GoalRow; onEdit: () => void; onDelete: () => void }) {
  const { t } = useLingui();
  const { goal, progress } = row;
  const currency = goal.currency;
  // Above the target the bar is full and the overshoot is shown as a figure, not a longer track.
  const share = Math.min(Math.max(toNumber(progress.progress) ?? 0, 0), 1);

  const pace =
    progress.required_monthly_base !== null ? (
      <Trans>
        <Money value={progress.required_monthly_base} currency={currency} /> a month to arrive on time
      </Trans>
    ) : progress.projected_date !== null ? (
      <Trans>at this pace, {formatDay(progress.projected_date)}</Trans>
    ) : progress.monthly_base !== null ? (
      <Trans>this pace does not arrive</Trans>
    ) : (
      <Trans>no monthly amount stated</Trans>
    );

  return (
    <ListRow
      box
      top
      title={goal.name}
      sub={row.account_names.length > 0 ? row.account_names.join(" · ") : <Trans>the whole portfolio</Trans>}
      value={<Money value={progress.current_base} currency={currency} />}
      meta={
        <Trans>
          of <Money value={progress.target_base} currency={currency} />
        </Trans>
      }
      end={
        <>
          <button type="button" className="iconbtn iconbtn--sm" aria-label={t`Edit`} onClick={onEdit}>
            <PencilSimpleIcon />
          </button>
          <button
            type="button"
            className="iconbtn iconbtn--sm iconbtn--danger"
            aria-label={t`Delete`}
            onClick={onDelete}
          >
            <TrashIcon />
          </button>
        </>
      }
      foot={
        <span className="stack">
          <Bar fill={`${share * 100}%`} size="sm" label={formatPercent(progress.progress)} />
          <span>
            {formatPercent(progress.progress)}
            {progress.months_left !== null && (
              <>
                {" · "}
                {plural(progress.months_left, { one: "# month left", other: "# months left" })}
              </>
            )}
            {progress.on_track !== null && (
              <>
                {" · "}
                <Badge tone={progress.on_track ? "in" : "warn"}>
                  {progress.on_track ? t`on track` : t`behind`}
                </Badge>
              </>
            )}
            {" · "}
            <span className="dim">{pace}</span>
          </span>
        </span>
      }
    />
  );
}

function GoalForm({
  draft,
  accounts,
  onChange,
  onSubmit,
  onClose,
  busy,
  error,
}: {
  draft: GoalInput;
  accounts: AccountRow[];
  onChange: (draft: GoalInput) => void;
  onSubmit: () => void;
  onClose: () => void;
  busy: boolean;
  error: Error | null;
}) {
  const { t } = useLingui();
  const toggle = (id: string, on: boolean) =>
    onChange({
      ...draft,
      accounts: on ? [...draft.accounts, id] : draft.accounts.filter((a) => a !== id),
    });

  return (
    <FormDialog
      title={draft.id ? t`Edit the goal` : t`New goal`}
      onClose={onClose}
      onSubmit={onSubmit}
      busy={busy}
      error={error}
      ready={draft.name.trim() !== "" && draft.target_amount.trim() !== ""}
    >
      <Field label={t`Name`}>
        <input value={draft.name} autoFocus onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>
      <FieldPair>
        <Field label={t`Target amount`}>
          <input
            inputMode="decimal"
            value={draft.target_amount}
            onChange={(e) => onChange({ ...draft, target_amount: e.target.value })}
          />
        </Field>
        <Field label={t`Currency`}>
          <input
            value={draft.currency}
            onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
          />
        </Field>
      </FieldPair>
      <FieldPair>
        <Field label={t`By`} hint={t`Leave empty to ask when the monthly amount would arrive instead.`}>
          <input
            type="date"
            value={draft.target_date ?? ""}
            onChange={(e) => onChange({ ...draft, target_date: e.target.value || null })}
          />
        </Field>
        <Field label={t`Paid in monthly`} hint={t`Optional: what you actually put aside.`}>
          <input
            inputMode="decimal"
            value={draft.monthly_amount ?? ""}
            onChange={(e) => onChange({ ...draft, monthly_amount: e.target.value || null })}
          />
        </Field>
      </FieldPair>
      <Field
        label={t`Expected return, % a year`}
        hint={t`An assumption, never this portfolio's measured return. Leave empty for pure saving.`}
      >
        <input
          inputMode="decimal"
          value={draft.expected_return ?? ""}
          onChange={(e) => onChange({ ...draft, expected_return: e.target.value || null })}
        />
      </Field>
      <Field label={t`Accounts that count`} hint={t`None ticked means the whole portfolio.`}>
        <div className="smenu">
          {accounts.map((account) => (
            <CheckField
              key={account.id}
              label={account.name}
              checked={draft.accounts.includes(account.id)}
              onChange={(on) => toggle(account.id, on)}
            />
          ))}
        </div>
      </Field>
    </FormDialog>
  );
}
