import { Trans, useLingui } from "@lingui/react/macro";
import { useEffect, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PlusIcon } from "@phosphor-icons/react";
import { api, today } from "../../lib/api";
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
import { formatDay, formatMoney } from "../../lib/format";
import { transactionLabel } from "../../lib/kinds";
import { affects, useAccounts, useInvalidate, useTransactions } from "../../lib/queries";
import type { TransactionFilter, TransactionInput, TransactionRow } from "../../lib/types";
import { Filters } from "./Filters";
import { JournalTable } from "./JournalTable";
import { TransactionForm } from "./TransactionForm";

export function Transactions({ focus }: { focus?: string | null }) {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const [filter, setFilter] = useState<TransactionFilter>({});
  const [query, setQuery] = useState("");

  // Navigation hints use the same visible search filter as typed input.
  useEffect(() => {
    if (focus) setQuery(focus);
  }, [focus]);
  const [draft, setDraft] = useState<TransactionInput | null>(null);
  const menu = useMenu();

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
  const needle = query.trim().toLowerCase();
  const shown = (rows.data?.rows ?? []).filter(
    (row) =>
      !needle ||
      (row.symbol ?? "").toLowerCase().includes(needle) ||
      (row.note ?? "").toLowerCase().includes(needle) ||
      transactionLabel(i18n, row.kind).toLowerCase().includes(needle) ||
      row.account_name.toLowerCase().includes(needle),
  );
  const selection = useSelection(shown.map((row) => row.id));

  if (accounts.isError) return <QueryError error={accounts.error} />;
  if (accounts.isPending) return <Pending />;

  // Purchases default to a securities account because cash accounts cannot hold them.
  const firstDepot = accounts.data.find((a) => a.kind === "SECURITIES");

  const blank = (): TransactionInput => ({
    id: null,
    account_id: firstDepot?.id ?? accounts.data[0]?.id ?? "",
    security_id: null,
    kind: "BUY",
    date: today(),
    quantity: null,
    price: null,
    amount: null,
    fees: null,
    taxes: null,
    currency: firstDepot?.currency ?? accounts.data[0]?.currency ?? "EUR",
    fee_currency: null,
    tax_currency: null,
    fx_rate_to_base: null,
    note: null,
  });

  const edit = (row: TransactionRow) =>
    setDraft({
      id: row.id,
      account_id: row.account_id,
      security_id: row.security_id,
      kind: row.kind,
      date: row.date,
      quantity: row.quantity,
      price: row.price,
      amount: row.amount,
      fees: row.fees,
      taxes: row.taxes,
      currency: row.currency,
      fee_currency: row.fee_currency,
      tax_currency: row.tax_currency,
      fx_rate_to_base: row.fx_rate_to_base,
      note: row.note,
    });

  const years = [...new Set((all.data?.rows ?? []).map((row) => row.date.slice(0, 4)))].sort(
    (a, b) => Number(b) - Number(a),
  );

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
      asOf={formatDay(today())}
      actions={
        <button className="btn" onClick={() => setDraft(blank())}>
          <PlusIcon /> <Trans>New transaction</Trans>
        </button>
      }
      filters={
        <Filters query={query} onQuery={setQuery} filter={filter} onFilter={setFilter} years={years} />
      }
    >
      <ErrorText error={remove.error} />

      {draft && (
        <TransactionForm
          draft={draft}
          accounts={accounts.data.map((a) => ({
            id: a.id,
            name: a.name,
            currency: a.currency,
            kind: a.kind,
          }))}
          securities={(all.data?.rows ?? [])
            .filter((row) => row.security_id)
            .reduce<Array<{ id: string; symbol: string }>>((list, row) => {
              if (!list.some((s) => s.id === row.security_id)) {
                list.push({ id: row.security_id!, symbol: row.symbol ?? row.security_id! });
              }
              return list;
            }, [])}
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
