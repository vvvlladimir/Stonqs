import { plural } from "@lingui/core/macro";
import { Command } from "../../lib/commands";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { formatMoney } from "../../lib/format";
import { affects, useAccountGroups, useAccounts, useAccountsTotal, useInvalidate } from "../../lib/queries";
import { Page } from "../../components/Page";
import { Empty, ErrorText, InfoHeading, List, Pending, QueryError, useMenu } from "../../components/ui";
import type { AccountInput, AccountRow } from "../../lib/types";
import { AccountCard } from "./AccountCard";
import { AccountForm } from "./AccountForm";
import { GroupsPanel } from "./GroupsPanel";
import { LimitsPanel } from "./LimitsPanel";
import { EMPTY_ACCOUNT } from "./model";

export function Accounts() {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const accounts = useAccounts();
  const groups = useAccountGroups();
  const total = useAccountsTotal();
  const [draft, setDraft] = useState<AccountInput | null>(null);
  const menu = useMenu();

  const save = useMutation({
    mutationFn: api.accountSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.accounts);
    },
  });

  const remove = useMutation({
    mutationFn: ({ id, force }: { id: string; force: boolean }) => api.accountDelete(id, force),
    onSuccess: () => invalidate(...affects.accounts),
  });

  if (accounts.isError) return <QueryError error={accounts.error} />;
  if (accounts.isPending) return <Pending />;

  const deposits = accounts.data.filter((a) => a.kind === "DEPOSIT");
  const depots = accounts.data.filter((a) => a.kind === "SECURITIES");

  const edit = (row: AccountRow) =>
    setDraft({
      id: row.id,
      name: row.name,
      currency: row.currency,
      kind: row.kind,
      reference_account_id: row.reference_account_id,
      is_active: row.is_active,
      opened_at: row.opened_at,
    });

  // Account deletion cascades transactions; show the count before confirming.
  const del = (row: AccountRow) => {
    const force =
      row.transaction_count === 0 ||
      confirm(
        t`Account "${row.name}" carries ${row.transaction_count} transactions. Delete them along with it?`,
      );
    if (force) remove.mutate({ id: row.id, force: true });
  };

  const newAccount = () =>
    setDraft({ ...EMPTY_ACCOUNT, currency: deposits[0]?.currency ?? EMPTY_ACCOUNT.currency });

  const base = total.data?.base_currency ?? "";
  // A dash means valuation lacks a quote or FX rate; zero would be misleading.
  const totalValue =
    total.data && total.data.value_base !== null
      ? formatMoney(total.data.value_base, total.data.base_currency)
      : null;

  const card = (row: AccountRow) => (
    <AccountCard key={row.id} row={row} base={base} menu={menu} onEdit={edit} onDelete={del} />
  );

  return (
    <Page
      archetype="registry"
      title={t`Accounts`}
      summary={[
        plural(accounts.data.length, { one: "# account", other: "# accounts" }),
        totalValue,
        plural(groups.data?.length ?? 0, { one: "# group", other: "# groups" }),
      ]
        .filter(Boolean)
        .join(" · ")}
      actions={
        <>
          <button className="btn" onClick={newAccount}>
            <PlusIcon /> <Trans>New account</Trans>
          </button>
          <Command id="new" label={t`New account`} run={newAccount} />
          <Command id="newAccount" run={newAccount} />
        </>
      }
    >
      <ErrorText error={remove.error} />

      {draft && (
        <AccountForm
          draft={draft}
          deposits={deposits}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onCancel={() => setDraft(null)}
          pending={save.isPending}
          error={save.error}
        />
      )}

      <section className="section">
        <div className="section__head">
          <InfoHeading
            title={t`Securities accounts`}
            info={t`Holds instruments; the money for its trades goes through the linked cash account.`}
          />
        </div>
        {depots.length === 0 ? (
          <Empty title={t`No securities accounts`}>
            <Trans>
              Instruments sit on a securities account; the money for its trades sits on the linked cash
              account.
            </Trans>
          </Empty>
        ) : (
          <List variant="grid">{depots.map(card)}</List>
        )}
      </section>

      <section className="section">
        <div className="section__head">
          <InfoHeading
            title={t`Cash accounts`}
            info={t`Holds money in one currency; several securities accounts can settle through the same one.`}
          />
        </div>
        {deposits.length === 0 ? (
          <Empty title={t`No cash accounts`}>
            <Trans>The portfolio starts here: a securities account cannot exist without one.</Trans>
          </Empty>
        ) : (
          <List variant="grid">{deposits.map(card)}</List>
        )}
      </section>

      <GroupsPanel accounts={accounts.data} base={base} />

      <LimitsPanel accounts={accounts.data} base={base} />

      {menu.node}
    </Page>
  );
}
