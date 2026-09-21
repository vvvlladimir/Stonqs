import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PlusIcon } from "@phosphor-icons/react";
import { Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import {
  Check,
  Empty,
  ErrorText,
  List,
  ListRow,
  Money,
  Panel,
  SelectionBar,
  useMenu,
  useSelection,
  type MenuItem,
} from "../../components/ui";
import { affects, useAccountGroups, useInvalidate } from "../../lib/queries";
import type { AccountGroupInput, AccountGroupRow, AccountRow } from "../../lib/types";
import { GroupForm } from "./GroupForm";
import { EMPTY_GROUP } from "./model";

/** Groups are saved slices of the portfolio, so they live beside the accounts they name. */
export function GroupsPanel({ accounts, base }: { accounts: AccountRow[]; base: string }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const groups = useAccountGroups();
  const [draft, setDraft] = useState<AccountGroupInput | null>(null);
  const menu = useMenu();
  const pick = useSelection((groups.data ?? []).map((g) => g.id));

  const save = useMutation({
    mutationFn: api.accountGroupSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.accounts);
    },
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.accountGroupDelete(id),
    onSuccess: () => invalidate(...affects.accounts),
  });

  // Groups do not own accounts, so bulk deletion can safely loop over rows.
  const removeMany = useMutation({
    mutationFn: async (ids: string[]) => {
      for (const id of ids) await api.accountGroupDelete(id);
    },
    onSettled: () => {
      pick.clear();
      invalidate(...affects.accounts);
    },
  });

  return (
    <Panel
      title={t`Groups`}
      tools={
        <button
          className="btn btn--ghost btn--sm"
          onClick={() => setDraft(EMPTY_GROUP)}
          disabled={accounts.length === 0}
        >
          <PlusIcon /> <Trans>New group</Trans>
        </button>
      }
      info={t`A saved set of accounts to pick as the data source; groups may overlap, and deleting one deletes no account.`}
    >
      <ErrorText error={remove.error} />
      <ErrorText error={removeMany.error} />

      {draft && (
        <GroupForm
          draft={draft}
          accounts={accounts}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onCancel={() => setDraft(null)}
          pending={save.isPending}
          error={save.error}
        />
      )}

      <SelectionBar count={pick.count} onClear={pick.clear}>
        <button
          type="button"
          className="iconbtn iconbtn--sm iconbtn--danger"
          disabled={removeMany.isPending}
          onClick={() => removeMany.mutate(pick.ids)}
        >
          {removeMany.isPending ? t`Deleting…` : t`Delete (${pick.count})`}
        </button>
      </SelectionBar>

      {groups.data && groups.data.length > 0 ? (
        <List>
          {groups.data.map((group: AccountGroupRow) => {
            const items: MenuItem[] = [
              {
                label: t`Edit…`,
                onSelect: () =>
                  setDraft({
                    id: group.id,
                    name: group.name,
                    account_ids: group.account_ids,
                  }),
              },
              { label: t`Delete`, danger: true, onSelect: () => remove.mutate(group.id) },
            ];
            return (
              <ListRow
                key={group.id}
                {...menu.row(group.id, items)}
                pick={
                  <Check
                    checked={pick.has(group.id)}
                    onChange={() => pick.toggle(group.id)}
                    label={t`Select "${group.name}"`}
                  />
                }
                title={group.name}
                sub={group.account_names.join(" · ") || t`no accounts`}
                value={group.value_base === null ? "—" : <Money value={group.value_base} currency={base} />}
                actions={menu.button(group.id, items)}
              />
            );
          })}
        </List>
      ) : (
        <Empty title={t`No groups`}>
          <Trans>
            A group is picked as the data source at the bottom of the navigation; every screen then counts its
            accounts only.
          </Trans>
        </Empty>
      )}
      {menu.node}
    </Panel>
  );
}
