import { plural } from "@lingui/core/macro";
import { Command } from "../../lib/commands";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { ColumnsIcon, PlusIcon } from "@phosphor-icons/react";
import { api } from "../../lib/api";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import { affects, useInvalidate, useWatchlistRows, useWatchlists } from "../../lib/queries";
import { useUiState } from "../../lib/uiState";
import { Page } from "../../components/Page";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { useSecurityCard } from "../../components/domain/SecurityCardProvider";
import {
  Async,
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
import { orderColumns } from "../../components/domain/positionColumns";
import { useWatchContext } from "./useWatchContext";
import { WatchColumnPicker } from "./WatchColumnPicker";
import { useAsOf } from "../../lib/asOf";

export function Watchlist() {
  const { t } = useLingui();
  const date = useAsOf().date;
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
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, period);
  const active = lists.data?.find((l) => l.id === chosen) ?? lists.data?.[0] ?? null;
  const rows = useWatchlistRows(active?.id ?? null, range);
  const { currency, allColumns, shownColumns, securities, ctx } = useWatchContext(
    date,
    range,
    ui.watch_columns,
  );

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

  const columns = orderColumns(allColumns, shownColumns);

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
          {active ? (
            <Command id="new" label={t`Add instruments`} run={() => setAdding(true)} />
          ) : (
            <Command id="new" label={t`New list`} run={newList} />
          )}
          <Command id="newWatchlist" run={newList} />
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
                    onSortChange={(watch_sort) => saveUi((ui) => ({ ...ui, watch_sort }))}
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

      {adding && active && securities && (
        <AddDialog list={active} securities={securities} onClose={() => setAdding(false)} />
      )}

      {picking && (
        <WatchColumnPicker
          columns={allColumns}
          selected={shownColumns}
          currency={currency}
          onClose={() => setPicking(false)}
        />
      )}

      {menu.node}
    </Page>
  );
}
