import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { Badge, Buttons, Empty, ErrorText, Field, List, ListRow } from "../../components/ui";
import type { SecurityDraft, SymbolMapping } from "../../lib/types";

/** Manual lookup for instruments with multiple valid listings. */
export function AssetSearch({
  symbol,
  onPick,
}: {
  symbol: SymbolMapping;
  onPick: (value: string, draft: SecurityDraft) => void;
}) {
  const { t } = useLingui();
  const [query, setQuery] = useState(symbol.isin ?? symbol.file_name ?? symbol.value);
  const search = useMutation({ mutationFn: (q: string) => api.securitySearch(q) });
  const pick = useMutation({
    mutationFn: (candidate: string) => api.importResolveSymbol(candidate, null, null, symbol.currency),
    onSuccess: (draft) => {
      if (!draft) return;
      // Preserve the source ISIN so repeated imports resolve the same instrument.
      onPick(symbol.value, { ...draft, isin: draft.isin ?? symbol.isin });
    },
  });

  return (
    <>
      <Field
        label={t`Search the directory`}
        hint={t`An ISIN finds the instrument; a ticker finds one listing`}
      >
        <Buttons>
          <input
            value={query}
            placeholder={t`ISIN, ticker or name`}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button
            type="button"
            className="btn btn--ghost"
            disabled={search.isPending || !query.trim()}
            onClick={() => search.mutate(query)}
          >
            {search.isPending ? t`Searching…` : t`Search`}
          </button>
        </Buttons>
      </Field>

      {search.data?.length === 0 && (
        <Empty title={t`Nothing found`}>
          <Trans>
            Try the ISIN, or the full name. The instrument can also be left as it is, with prices entered by
            hand.
          </Trans>
        </Empty>
      )}

      {search.data !== undefined && search.data.length > 0 && (
        <List>
          {search.data.map((found) => (
            <ListRow
              key={`${found.source}:${found.symbol}`}
              wrap
              title={<span className="mono">{found.symbol}</span>}
              sub={
                <span className="inline">
                  <Badge>{found.kind}</Badge>
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
                  disabled={pick.isPending}
                  onClick={() => pick.mutate(found.symbol)}
                >
                  <Trans>choose</Trans>
                </button>
              }
            />
          ))}
        </List>
      )}

      <ErrorText error={search.error} />
      <ErrorText error={pick.error} />
    </>
  );
}
