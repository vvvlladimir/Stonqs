import { useLingui } from "@lingui/react/macro";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { affects, useInvalidate } from "../../lib/queries";
import { useSecurityCard } from "../../components/domain/SecurityCardProvider";
import type { MenuItem } from "../../components/ui";
import type { Watchlist, WatchlistInput, WatchRow } from "../../lib/types";

/** What the screen writes, and the two context menus that write it: a row's and a list tab's. */
export function useWatchlistActions({
  lists,
  active,
  onChosen,
  onDraft,
  onAdd,
}: {
  lists: Watchlist[] | undefined;
  active: Watchlist | null;
  onChosen: (id: string | null) => void;
  onDraft: (draft: WatchlistInput | null) => void;
  onAdd: () => void;
}) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const card = useSecurityCard();

  const save = useMutation({
    mutationFn: api.watchlistSave,
    onSuccess: (list) => {
      onDraft(null);
      onChosen(list.id);
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
      onChosen(null);
      invalidate(...affects.watchlists);
    },
  });

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
    const list = lists?.find((l) => l.id === id);
    if (!list) return [];
    return [
      {
        label: t`Add instruments…`,
        onSelect: () => {
          onChosen(id);
          onAdd();
        },
      },
      { label: t`Rename…`, onSelect: () => onDraft({ ...list }) },
      { label: t`Delete`, danger: true, onSelect: () => del(list), disabled: remove.isPending },
    ];
  };

  return { save, edit, remove, itemsFor, listItems };
}
