import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { ColumnsIcon, PlusIcon } from "@phosphor-icons/react";
import { api, today } from "../../lib/api";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import {
  affects,
  useInvalidate,
  usePortfolio,
  usePositionReturns,
  usePositions,
  usePositionsCostBasis,
  useSecurities,
  useWatchlistRows,
  useWatchlists,
} from "../../lib/queries";
import { DEFAULT_UI, useUiState } from "../../lib/uiState";
import { formatDay } from "../../lib/format";
import { Page } from "../../components/Page";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { useSecurityCard } from "../../components/domain/SecurityCardProvider";
import {
  Async,
  ColumnPicker,
  Empty,
  ErrorText,
  Panel,
  Pending,
  QueryError,
  Tabs,
  useMenu,
  type MenuItem,
} from "../../components/ui";
import type { WatchlistInput, WatchRow } from "../../lib/types";
import { AddDialog } from "./AddDialog";
import { ListDialog } from "./ListDialog";
import { WatchTable } from "./WatchTable";
import { useWatchColumns } from "./columns";
import {
  GROUPS,
  needsCostQuery,
  orderColumns,
  resolveColumnIds,
} from "../../components/domain/positionColumns";

export function Watchlist() {
  const { t, i18n } = useLingui();
  const date = today();
  const invalidate = useInvalidate();
  const menu = useMenu();
  const card = useSecurityCard();
  const { ui, save: saveUi } = useUiState();
  const [chosen, setChosen] = useState<string | null>(null);
  // A watched instrument has no inception of its own in the portfolio, so a year is the default.
  const [period, setPeriod] = useState<PeriodId>("ONE_YEAR");
  const [draft, setDraft] = useState<WatchlistInput | null>(null);
  const [adding, setAdding] = useState(false);
  const [picking, setPicking] = useState(false);

  const lists = useWatchlists();
  const securities = useSecurities();
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, period);
  const active = lists.data?.find((l) => l.id === chosen) ?? lists.data?.[0] ?? null;
  const rows = useWatchlistRows(active?.id ?? null, range);
  // A held instrument also shows its position: the positions screen's own queries, shared.
  const positions = usePositions(date);
  const returns = usePositionReturns(range);
  const portfolio = usePortfolio();
  const allColumns = useWatchColumns();
  const method = portfolio.data?.cost_basis_method;
  // Same rule as the positions table: only a purchase figure under the *other* method is
  // worth the host's second holdings pass.
  const shownColumns = resolveColumnIds(ui.watch_columns, method);
  const costs = usePositionsCostBasis(needsCostQuery(shownColumns, method) ? date : null);

  const save = useMutation({
    mutationFn: api.watchlistSave,
    onSuccess: (list) => {
      setDraft(null);
      setChosen(list.id);
      invalidate(...affects.watchlists);
    },
  });
  const edit = useMutation({
    mutationFn: api.watchlistSave,
    onSuccess: () => invalidate(...affects.watchlists),
  });
  const remove = useMutation({
    mutationFn: api.watchlistDelete,
    onSuccess: () => {
      setChosen(null);
      invalidate(...affects.watchlists);
    },
  });

  if (lists.isError) return <QueryError error={lists.error} />;
  if (!lists.data) return <Pending />;

  const currency = portfolio.data?.base_currency ?? "";
  const columns = orderColumns(allColumns, shownColumns);
  const positionOf = new Map((positions.data?.rows ?? []).map((r) => [r.security_id, r]));
  const returnOf = new Map((returns.data ?? []).map((r) => [r.security_id, r]));
  const securityOf = new Map((securities.data ?? []).map((s) => [s.id, s]));
  const costOf = new Map((costs.data?.rows ?? []).map((r) => [r.security_id, r]));
  const ctx = (row: WatchRow) => ({
    currency,
    i18n,
    from: range?.from,
    position: positionOf.get(row.security_id),
    period: returnOf.get(row.security_id),
    security: securityOf.get(row.security_id),
    cost: costOf.get(row.security_id),
    own: method,
  });

  const newList = () => setDraft({ name: "", security_ids: [] });
  const ids = active?.security_ids ?? [];

  const move = (row: WatchRow, by: number) => {
    if (!active) return;
    const next = [...ids];
    const at = next.indexOf(row.security_id);
    [next[at], next[at + by]] = [next[at + by], next[at]];
    edit.mutate({ ...active, security_ids: next });
  };

  const itemsFor = (row: WatchRow): MenuItem[] => {
    const at = ids.indexOf(row.security_id);
    return [
      { label: t`Instrument card…`, onSelect: () => card.open(row.security_id) },
      { label: t`Move up`, onSelect: () => move(row, -1), disabled: at <= 0 },
      { label: t`Move down`, onSelect: () => move(row, 1), disabled: at < 0 || at >= ids.length - 1 },
      { label: t`Copy ticker`, onSelect: () => navigator.clipboard.writeText(row.symbol) },
      {
        label: t`Remove from the list`,
        danger: true,
        onSelect: () =>
          active && edit.mutate({ ...active, security_ids: ids.filter((id) => id !== row.security_id) }),
      },
    ];
  };

  const del = (list: { id: string; name: string }) => {
    if (confirm(t`Delete the watchlist "${list.name}"? The instruments stay in the directory.`)) {
      remove.mutate(list.id);
    }
  };

  /** A list's own actions live on its tab's context menu. */
  const listItems = (id: string): MenuItem[] => {
    const list = lists.data.find((l) => l.id === id);
    if (!list) return [];
    return [
      {
        label: t`Add instruments…`,
        onSelect: () => {
          setChosen(id);
          setAdding(true);
        },
      },
      { label: t`Rename…`, onSelect: () => setDraft({ ...list }) },
      { label: t`Delete`, danger: true, onSelect: () => del(list), disabled: remove.isPending },
    ];
  };

  const addButton = (
    <button type="button" className="btn btn--ghost" onClick={() => setAdding(true)}>
      <PlusIcon /> <Trans>Add instruments</Trans>
    </button>
  );

  return (
    <Page
      archetype="registry"
      title={t`Watchlist`}
      summary={[
        plural(lists.data.length, { one: "# list", other: "# lists" }),
        active ? plural(ids.length, { one: "# instrument", other: "# instruments" }) : null,
      ]
        .filter(Boolean)
        .join(" · ")}
      asOf={formatDay(date)}
      controls={<PeriodControl value={period} onChange={setPeriod} ranges={ranges.data} />}
      actions={
        <>
          <button type="button" className="iconbtn" onClick={() => setPicking(true)}>
            <ColumnsIcon /> <Trans>Columns</Trans>
          </button>
          {active && addButton}
          <button type="button" className="btn" onClick={newList}>
            <PlusIcon /> <Trans>New list</Trans>
          </button>
        </>
      }
    >
      <ErrorText error={remove.error} />
      <ErrorText error={edit.error} />

      {!active ? (
        <Empty
          title={t`No watchlists yet`}
          action={
            <button type="button" className="btn" onClick={newList}>
              <PlusIcon /> <Trans>New list</Trans>
            </button>
          }
        >
          <Trans>
            A watchlist follows instruments you do not have to hold: the last price, the period's move and the
            nearest alert level, side by side.
          </Trans>
        </Empty>
      ) : (
        <Tabs
          items={lists.data.map((l) => ({ id: l.id, label: l.name }))}
          value={active.id}
          onChange={setChosen}
          onMenu={(id, tab) => menu.openFrom(`list:${id}`, listItems(id), tab)}
          label={t`Watchlists`}
        >
          {ids.length === 0 ? (
            <Empty title={t`The list is empty`} action={addButton}>
              <Trans>Pick instruments from the directory, or find one online.</Trans>
            </Empty>
          ) : !range ? (
            <Pending />
          ) : (
            <Async query={rows}>
              {(data) => (
                <Panel table>
                  <WatchTable
                    rows={data}
                    columns={columns}
                    currency={currency}
                    ctx={ctx}
                    sort={ui.watch_sort}
                    onSortChange={(watch_sort) => saveUi({ ...ui, watch_sort })}
                    menu={menu}
                    itemsFor={itemsFor}
                  />
                </Panel>
              )}
            </Async>
          )}
        </Tabs>
      )}

      {draft && (
        <ListDialog
          draft={draft}
          onChange={setDraft}
          onSubmit={() => save.mutate(draft)}
          onCancel={() => setDraft(null)}
          pending={save.isPending}
          error={save.error}
        />
      )}

      {adding && active && securities.data && (
        <AddDialog list={active} securities={securities.data} onClose={() => setAdding(false)} />
      )}

      {picking && (
        <ColumnPicker
          columns={allColumns.map((c) => ({
            id: c.id,
            label: c.label(i18n, currency),
            group: c.group,
            tip: c.tip?.(i18n),
          }))}
          groups={GROUPS.map((g) => ({ id: g.id, label: i18n._(g.label) }))}
          selected={shownColumns}
          onChange={(watch_columns) => saveUi({ ...ui, watch_columns })}
          onReset={() => saveUi({ ...ui, watch_columns: DEFAULT_UI.watch_columns })}
          onClose={() => setPicking(false)}
        />
      )}

      {menu.node}
    </Page>
  );
}
