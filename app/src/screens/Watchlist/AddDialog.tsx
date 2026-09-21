import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { securityKindLabel } from "../../lib/kinds";
import { affects, useInvalidate } from "../../lib/queries";
import { Buttons, Check, Empty, ErrorText, List, ListRow, Modal, SearchBox, Tag } from "../../components/ui";
import type { SecurityMatch, SecurityRow, Watchlist } from "../../lib/types";

/**
 * Puts instruments on one list. One list of matches: the directory's first, then what the provider
 * found that the directory does not hold yet. Enter searches the provider.
 */
export function AddDialog({
  list,
  securities,
  onClose,
}: {
  list: Watchlist;
  securities: SecurityRow[];
  onClose: () => void;
}) {
  const { t, i18n } = useLingui();
  const invalidate = useInvalidate();
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const on = (id: string) => list.security_ids.includes(id);

  const toggle = useMutation({
    mutationFn: (security_ids: string[]) => api.watchlistSave({ ...list, security_ids }),
    onSuccess: () => invalidate(...affects.watchlists),
  });
  const search = useMutation({ mutationFn: (q: string) => api.securitySearch(q) });
  const run = () => {
    if (needle) search.mutate(query.trim());
  };

  /** Created first, then put on the list; saving it starts the fetch of its quotes. The currency
   * and the venue come from the picked listing's own profile; a listing without a currency is
   * refused, not guessed. */
  const add = useMutation({
    mutationFn: async (found: SecurityMatch) => {
      const profile = await api.securityProfile(found.source, found.symbol);
      if (!profile?.currency || profile.has_history === false) {
        throw new Error(
          t`${found.symbol} has no price history at the provider, so its currency is unknown. Pick another listing.`,
        );
      }
      const saved = await api.securitySave({
        id: null,
        symbol: found.symbol,
        name: found.name,
        currency: profile.currency,
        kind: found.kind,
        isin: found.isin ?? profile.isin,
        mic: profile.mic ?? found.mic,
        data_source: found.source,
        data_symbol: null,
        quantity_step: null,
        wkn: null,
        note: null,
      });
      // The host fetches the new instrument's quotes itself, as it does for every new one.
      await api.watchlistSave({ ...list, security_ids: [...list.security_ids, saved.id] });
    },
    onSuccess: () => invalidate(...affects.securities, ...affects.watchlists),
  });

  const local = securities.filter(
    (s) =>
      !needle ||
      s.symbol.toLowerCase().includes(needle) ||
      s.name.toLowerCase().includes(needle) ||
      (s.isin ?? "").toLowerCase().includes(needle),
  );
  // Results of an earlier query are not shown under a new one, and what the directory already
  // holds is listed once, above, with its box.
  const known = new Set(securities.map((s) => s.symbol.toUpperCase()));
  const searched = search.data !== undefined && search.variables === query.trim();
  const remote = searched ? search.data.filter((found) => !known.has(found.symbol.toUpperCase())) : [];

  return (
    <Modal title={t`Add to "${list.name}"`} onClose={onClose} wide>
      <Buttons>
        <SearchBox value={query} onChange={setQuery} onEnter={run} placeholder={t`Ticker, name or ISIN`} />
        <button type="button" className="btn btn--ghost" disabled={search.isPending || !needle} onClick={run}>
          {search.isPending ? t`Searching…` : t`Search the provider`}
        </button>
      </Buttons>

      {local.length === 0 && remote.length === 0 ? (
        <Empty title={searched ? t`Nothing found` : t`Not in the directory`}>
          {searched ? (
            <Trans>Try the ISIN, or the full name.</Trans>
          ) : (
            <Trans>Press Enter to search the provider.</Trans>
          )}
        </Empty>
      ) : (
        <List>
          {local.map((s) => (
            <ListRow
              key={s.id}
              pick={
                <Check
                  checked={on(s.id)}
                  label={s.symbol}
                  onChange={() =>
                    toggle.mutate(
                      on(s.id) ? list.security_ids.filter((id) => id !== s.id) : [...list.security_ids, s.id],
                    )
                  }
                />
              }
              title={<span className="mono">{s.symbol}</span>}
              sub={
                <span className="inline">
                  <Tag>{securityKindLabel(i18n, s.kind)}</Tag>
                  <span>{s.name}</span>
                </span>
              }
            />
          ))}
          {remote.map((found) => (
            <ListRow
              key={`${found.source}:${found.symbol}`}
              wrap
              title={<span className="mono">{found.symbol}</span>}
              sub={
                <span className="inline">
                  <Tag>{securityKindLabel(i18n, found.kind)}</Tag>
                  <span>
                    {found.name}
                    {found.exchange ? ` · ${found.exchange}` : ""}
                  </span>
                </span>
              }
              end={
                <button
                  type="button"
                  className="btn btn--sm"
                  disabled={add.isPending}
                  onClick={() => add.mutate(found)}
                >
                  {add.isPending && add.variables?.symbol === found.symbol ? t`Adding…` : t`Add`}
                </button>
              }
            />
          ))}
        </List>
      )}

      <ErrorText error={toggle.error} />
      <ErrorText error={search.error} />
      <ErrorText error={add.error} />
    </Modal>
  );
}
