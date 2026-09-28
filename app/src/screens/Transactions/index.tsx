import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { DownloadSimpleIcon, PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { ariaKeys, Command } from "../../lib/commands";
import { Page } from "../../components/Page";
import {
  Async,
  Empty,
  ErrorText,
  Panel,
  Pending,
  QueryError,
  SelectionBar,
  useMenu,
  useSelection,
  type MenuItem,
} from "../../components/ui";
import { formatMoney } from "../../lib/format";
import { affects, useAccounts, useInvalidate, useTransactions } from "../../lib/queries";
import type { TransactionFilter, TransactionInput, TransactionRow } from "../../lib/types";
import { blankTransaction, draftOf, matching, securitiesIn, yearsOf } from "./model";
import { Filters } from "./Filters";
import { JournalTable } from "./JournalTable";
import { TransactionForm } from "./TransactionForm";
import { useExport } from "./useExport";

export function Transactions({ focus }: { focus?: string | null }) {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const [filter, setFilter] = useState<TransactionFilter>({});
  // Navigation hints use the same visible search filter as typed input. The screen is keyed by
  // the hint (`App`), so arriving with a new one starts here rather than syncing in an effect.
  const [query, setQuery] = useState(focus ?? "");
  const [draft, setDraft] = useState<TransactionInput | null>(null);
  const menu = useMenu();
  const exported = useExport(filter, menu);

  const accounts = useAccounts();
  const rows = useTransactions(filter);
  // The full journal supplies filter years and the selection denominator.
  const all = useTransactions({});

  const save = useMutation({
    mutationFn: api.transactionSave,
    onSuccess: () => {
      setDraft(null);
      invalidate(...affects.transactions);
    },
  });
  const remove = useMutation({
    mutationFn: api.transactionDelete,
    onSuccess: () => invalidate(...affects.transactions),
  });

  /** Delete selected transactions through the single-row host command. */
  const removeMany = useMutation({
    mutationFn: async (ids: string[]) => {
      for (const id of ids) await api.transactionDelete(id);
    },
    onSettled: () => {
      selection.clear();
      invalidate(...affects.transactions);
    },
  });

  // Search stays client-side; server filters also change the core's monthly totals.
  const shown = (rows.data?.rows ?? []).filter(matching(i18n, query));
  const selection = useSelection(shown.map((row) => row.id));

  if (accounts.isError) return <QueryError error={accounts.error} />;
  if (accounts.isPending) return <Pending />;

  const blank = () => blankTransaction(accounts.data);
  const edit = (row: TransactionRow) => setDraft(draftOf(row));

  const years = yearsOf(all.data?.rows ?? []);

  const currency = rows.data?.base_currency ?? "";

  const itemsFor = (row: TransactionRow): MenuItem[] => [
    { label: t`Edit…`, onSelect: () => edit(row) },
    { label: t`Delete`, danger: true, onSelect: () => remove.mutate(row.id) },
  ];

  return (
    <Page
      archetype="registry"
      title={t`Transactions`}
      summary={
        rows.data
          ? t`${shown.length} of ${all.data?.rows.length ?? shown.length} · net ${formatMoney(
              rows.data.total_net,
              currency,
              { signed: true },
            )}`
          : undefined
      }
      actions={
        <>
          <button
            className="btn"
            aria-haspopup="menu"
            onClick={(e) => exported.choose(e.currentTarget)}
            disabled={exported.exporting}
          >
            <DownloadSimpleIcon /> <Trans>Export</Trans>
          </button>
          <button
            className="btn"
            aria-keyshortcuts={ariaKeys("newTransaction")}
            onClick={() => setDraft(blank())}
          >
            <PlusIcon /> <Trans>New transaction</Trans>
          </button>
          <Command id="newTransaction" run={() => setDraft(blank())} />
          <Command id="exportTransactions" run={exported.saveOwn} disabled={exported.exporting} />
          <Command id="new" label={t`New transaction`} run={() => setDraft(blank())} />
        </>
      }
      filters={
        <Filters query={query} onQuery={setQuery} filter={filter} onFilter={setFilter} years={years} />
      }
    >
      <ErrorText error={remove.error} />
      <ErrorText error={exported.error} />

      {draft && (
        <TransactionForm
          draft={draft}
          accounts={accounts.data.map((a) => ({
            id: a.id,
            name: a.name,
            currency: a.currency,
            kind: a.kind,
          }))}
          securities={securitiesIn(all.data?.rows ?? [])}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onCancel={() => setDraft(null)}
          pending={save.isPending}
          error={save.error}
        />
      )}

      <SelectionBar count={selection.count} onClear={selection.clear}>
        <button
          type="button"
          className="iconbtn iconbtn--sm iconbtn--danger"
          disabled={removeMany.isPending}
          onClick={() => removeMany.mutate(selection.ids)}
        >
          {removeMany.isPending ? t`Deleting…` : t`Delete (${selection.count})`}
        </button>
      </SelectionBar>
      <ErrorText error={removeMany.error} />

      <Async
        query={rows}
        isEmpty={() => shown.length === 0}
        empty={
          <Empty title={t`Nothing found`}>
            <Trans>
              Either there are no transactions yet, or the filter hid them. The first one is added with "New
              transaction"; a whole broker export goes through the "Import" screen.
            </Trans>
          </Empty>
        }
      >
        {(data) => (
          <Panel table>
            <JournalTable
              data={data}
              rows={shown}
              currency={currency}
              selection={selection}
              menu={menu}
              itemsFor={itemsFor}
            />
          </Panel>
        )}
      </Async>
      {menu.node}
    </Page>
  );
}
