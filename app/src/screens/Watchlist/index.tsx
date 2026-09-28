import { plural } from "@lingui/core/macro";
import { Command } from "../../lib/commands";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { ColumnsIcon, PlusIcon } from "@phosphor-icons/react";
import { pickRange, usePeriodRanges, type PeriodId } from "../../lib/periods";
import { useWatchlistRows, useWatchlists } from "../../lib/queries";
import { useUiState } from "../../lib/uiState";
import { Page } from "../../components/Page";
import { PeriodControl } from "../../components/domain/PeriodControl";
import { Async, Empty, ErrorText, Panel, Pending, QueryError, Tabs, useMenu } from "../../components/ui";
import type { WatchlistInput } from "../../lib/types";
import { useWatchlistActions } from "./useWatchlistActions";
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
  const menu = useMenu();
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

  const { save, edit, remove, itemsFor, listItems } = useWatchlistActions({
    lists: lists.data,
    active,
    onChosen: setChosen,
    onDraft: setDraft,
    onAdd: () => setAdding(true),
  });

  if (lists.isError) return <QueryError error={lists.error} />;
  if (!lists.data) return <Pending />;

  const columns = orderColumns(allColumns, shownColumns);

  const newList = () => setDraft({ name: "", security_ids: [] });
  const ids = active?.security_ids ?? [];

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
