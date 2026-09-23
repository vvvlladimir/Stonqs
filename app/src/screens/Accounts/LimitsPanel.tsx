import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PencilSimpleIcon, PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";

import { api } from "../../lib/api";
import { useAsOf } from "../../lib/asOf";
import { formatDay, formatPercent, toNumber } from "../../lib/format";
import { affects, useInvalidate, useLimits } from "../../lib/queries";
import {
  Async,
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
import type { AccountRow, LimitInput, LimitUsage } from "../../lib/types";

const BLANK: LimitInput = {
  id: null,
  account_id: "",
  name: "",
  amount: "",
  currency: "",
  year_starts_on: "01-01",
  withdrawals_restore: false,
  note: null,
};

/**
 * A yearly contribution ceiling — an ISA, a 401(k), an ИИС. It is measured and never enforced:
 * nothing here refuses a transaction, and the app ships no country's rules of its own.
 */
export function LimitsPanel({ accounts, base }: { accounts: AccountRow[]; base: string }) {
  const { t } = useLingui();
  const date = useAsOf().date;
  const invalidate = useInvalidate();
  const limits = useLimits(date);
  const [draft, setDraft] = useState<LimitInput | null>(null);

  const save = useMutation({
    mutationFn: api.limitSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.goals);
    },
  });
  const remove = useMutation({
    mutationFn: api.limitDelete,
    onSuccess: () => invalidate(...affects.goals),
  });

  const edit = (usage: LimitUsage) =>
    setDraft({
      id: usage.limit_id,
      account_id: usage.account_id,
      name: usage.name,
      amount: usage.allowance,
      currency: usage.currency,
      // The stored day is the one the year opens on; the reading's `from` carries its year too.
      year_starts_on: usage.from.slice(5),
      withdrawals_restore: usage.withdrawals_restore,
      note: null,
    });

  const del = (usage: LimitUsage) => {
    if (confirm(t`Delete the limit "${usage.name}"? No transaction changes.`)) remove.mutate(usage.limit_id);
  };

  const nameOf = (id: string) => accounts.find((a) => a.id === id)?.name ?? id;

  return (
    <Panel
      title={t`Contribution limits`}
      info={t`What one account may take in a year, against what it has taken. Nothing is blocked.`}
      tools={
        <button
          type="button"
          className="btn btn--sm"
          disabled={accounts.length === 0}
          onClick={() => setDraft({ ...BLANK, account_id: accounts[0]?.id ?? "", currency: base })}
        >
          <PlusIcon /> <Trans>New limit</Trans>
        </button>
      }
    >
      <ErrorText error={remove.error} />
      <Async
        query={limits}
        empty={
          <Empty title={t`No limits set`}>
            <Trans>
              A limit is a yearly ceiling on what one account may take — an ISA, a 401(k), an ИИС. You state
              the amount and the day its year opens; the app counts what was paid in and never blocks
              anything.
            </Trans>
          </Empty>
        }
      >
        {(rows) => (
          <List variant="cards">
            {rows.map((usage) => (
              <LimitCard
                key={usage.limit_id}
                usage={usage}
                account={nameOf(usage.account_id)}
                onEdit={() => edit(usage)}
                onDelete={() => del(usage)}
              />
            ))}
          </List>
        )}
      </Async>

      {draft && (
        <LimitForm
          draft={draft}
          accounts={accounts}
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

function LimitCard({
  usage,
  account,
  onEdit,
  onDelete,
}: {
  usage: LimitUsage;
  account: string;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t } = useLingui();
  const share = Math.min(Math.max(toNumber(usage.share) ?? 0, 0), 1);
  const over = (toNumber(usage.share) ?? 0) > 1;

  return (
    <ListRow
      box
      top
      title={usage.name}
      sub={account}
      value={<Money value={usage.used} currency={usage.currency} />}
      meta={
        <Trans>
          of <Money value={usage.allowance} currency={usage.currency} />
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
          <Bar fill={`${share * 100}%`} size="sm" tone={over ? "neg" : undefined} />
          <span>
            {formatPercent(usage.share)}
            {" · "}
            <Trans>
              <Money value={usage.remaining} currency={usage.currency} /> left
            </Trans>
            {" · "}
            <span className="dim">
              <Trans>
                year {formatDay(usage.from)} — {formatDay(usage.to)}
              </Trans>
            </span>
          </span>
        </span>
      }
    />
  );
}

function LimitForm({
  draft,
  accounts,
  onChange,
  onSubmit,
  onClose,
  busy,
  error,
}: {
  draft: LimitInput;
  accounts: AccountRow[];
  onChange: (draft: LimitInput) => void;
  onSubmit: () => void;
  onClose: () => void;
  busy: boolean;
  error: Error | null;
}) {
  const { t } = useLingui();

  return (
    <FormDialog
      title={draft.id ? t`Edit the limit` : t`New limit`}
      onClose={onClose}
      onSubmit={onSubmit}
      busy={busy}
      error={error}
      ready={draft.name.trim() !== "" && draft.amount.trim() !== "" && draft.account_id !== ""}
    >
      <Field label={t`Name`}>
        <input value={draft.name} autoFocus onChange={(e) => onChange({ ...draft, name: e.target.value })} />
      </Field>
      <Field
        label={t`Account`}
        options={accounts.map((a) => ({ value: a.id, label: a.name }))}
        value={draft.account_id}
        onChange={(account_id) => onChange({ ...draft, account_id })}
      />
      <FieldPair>
        <Field label={t`Allowance`}>
          <input
            inputMode="decimal"
            value={draft.amount}
            onChange={(e) => onChange({ ...draft, amount: e.target.value })}
          />
        </Field>
        <Field label={t`Currency`}>
          <input
            value={draft.currency}
            onChange={(e) => onChange({ ...draft, currency: e.target.value.toUpperCase() })}
          />
        </Field>
      </FieldPair>
      <Field
        label={t`The year opens on`}
        hint={t`Month and day, as MM-DD. Not every allowance year is the calendar one.`}
      >
        <input
          value={draft.year_starts_on}
          placeholder="01-01"
          onChange={(e) => onChange({ ...draft, year_starts_on: e.target.value })}
        />
      </Field>
      <CheckField
        label={t`Withdrawals give allowance back`}
        hint={t`On for a flexible allowance. Off, money taken out still counts as paid in.`}
        checked={draft.withdrawals_restore}
        onChange={(withdrawals_restore) => onChange({ ...draft, withdrawals_restore })}
      />
    </FormDialog>
  );
}
