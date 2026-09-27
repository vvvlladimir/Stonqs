import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PlusIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";

import { api } from "../../lib/api";
import { useAsOf } from "../../lib/asOf";
import { affects, useInvalidate, useLimits } from "../../lib/queries";
import { Async, Empty, ErrorText, List, Panel } from "../../components/ui";
import type { AccountRow, LimitInput, LimitUsage } from "../../lib/types";
import { LimitCard } from "./LimitCard";
import { LimitForm } from "./LimitForm";

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
