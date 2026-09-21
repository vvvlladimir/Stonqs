import { Trans, useLingui } from "@lingui/react/macro";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { affects, useInvalidate, useWatchlists } from "../../lib/queries";
import { Async, CheckField, ErrorText, Modal } from "../ui";
import type { Watchlist } from "../../lib/types";

/** Which lists one instrument is on; a box saves its list at once. */
export function WatchlistPicker({ securityId, onClose }: { securityId: string; onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const lists = useWatchlists();
  const toggle = useMutation({
    mutationFn: ({ list, on }: { list: Watchlist; on: boolean }) =>
      api.watchlistSave({
        ...list,
        security_ids: on
          ? [...list.security_ids, securityId]
          : list.security_ids.filter((id) => id !== securityId),
      }),
    onSuccess: () => invalidate(...affects.watchlists),
  });

  return (
    <Modal title={t`Watchlists`} onClose={onClose}>
      <Async
        query={lists}
        isEmpty={(data) => data.length === 0}
        empty={
          <p className="muted">
            <Trans>No watchlist yet: create one on the Watchlist screen.</Trans>
          </p>
        }
      >
        {(data) =>
          data.map((list) => (
            <CheckField
              key={list.id}
              label={list.name}
              checked={list.security_ids.includes(securityId)}
              disabled={toggle.isPending}
              onChange={(on) => toggle.mutate({ list, on })}
            />
          ))
        }
      </Async>
      <ErrorText error={toggle.error} />
    </Modal>
  );
}
