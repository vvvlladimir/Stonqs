import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../lib/api";
import { affects, keys, useInvalidate, useListings } from "../../lib/queries";
import { Buttons, DataTable, ErrorText, Modal, Money } from "../ui";
import { toNumber } from "../../lib/format";
import type { Listing, SecurityRow } from "../../lib/types";

/**
 * Choose which venue to price a security from — one instrument trades on a dozen
 * exchanges, each with its own currency, and there's no "correct" one except the
 * one the broker actually used.
 */
export function ListingPicker({ row, onClose }: { row: SecurityRow; onClose: () => void }) {
  const { t } = useLingui();
  const queryClient = useQueryClient();
  const invalidate = useInvalidate();

  // Fetch listings only when this picker is open; the request uses the network.
  const listings = useListings(row.id);

  const refresh = useMutation({
    mutationFn: () => api.securityListings(row.id, true),
    onSuccess: (fresh) => queryClient.setQueryData(keys.listings(row.id), fresh),
  });

  const choose = useMutation({
    mutationFn: (listing: Listing) =>
      api.securitySetListing({
        security_id: row.id,
        symbol: listing.symbol ?? "",
        currency: listing.currency ?? "",
        // Keep the venue with the symbol; a ticker alone is not globally unique.
        mic: listing.mic,
      }),
    onSuccess: () => {
      invalidate(...affects.securities);
      onClose();
    },
  });

  // A matching symbol is not a chosen listing: the venue may still be unset, and choosing is
  // exactly what fills it. Only both together mean there is nothing left to pick.
  const isCurrent = (listing: Listing) => listing.symbol === row.symbol && listing.mic === row.mic;

  const rows = listings.data ?? [];
  const usable = rows.filter((l) => l.has_history !== false);
  const dead = rows.length - usable.length;

  return (
    <Modal
      wide
      onClose={onClose}
      title={
        <>
          <Trans>Venues · {row.symbol}</Trans>
          {row.isin && <span className="sub"> {row.isin}</span>}
        </>
      }
    >
      <section className="stack">
        <p className="muted">
          <Trans>
            One instrument trades on many exchanges and in many currencies. Pick the venue your broker
            actually traded on: price and cost basis then share one currency, and the valuation stops going
            through an exchange rate every day. Switching a venue deletes the quotes already downloaded — the
            new one has a different price series.
          </Trans>
        </p>

        {listings.isPending && (
          <p className="muted">
            <Trans>Asking the directory…</Trans>
          </p>
        )}
        <ErrorText error={listings.error} />
        <ErrorText error={choose.error} />
        <ErrorText error={refresh.error} />

        {listings.isSuccess && rows.length === 0 && (
          <p className="muted">
            {row.isin ? (
              <Trans>
                The directory knows no other venue for this ISIN. A ticker can still be set by hand under
                "Edit".
              </Trans>
            ) : (
              <Trans>
                This instrument has no ISIN, so the venues were looked for under its ticker alone and the
                provider named none. Adding the ISIN under "Edit" asks the directory proper.
              </Trans>
            )}
          </p>
        )}

        {usable.length > 0 && (
          <DataTable
            card={false}
            rows={usable}
            rowKey={(listing) => `${listing.mic}-${listing.ticker}`}
            columns={[
              {
                key: "symbol",
                sort: (listing) => listing.symbol,
                header: t`Symbol`,
                align: "left",
                className: "nm",
                cell: (listing) => (
                  <>
                    {listing.symbol}
                    {isCurrent(listing) && (
                      <span className="badge">
                        <Trans>current</Trans>
                      </span>
                    )}
                  </>
                ),
              },
              {
                key: "exchange",
                sort: (listing) => listing.exchange ?? listing.mic,
                header: t`Exchange`,
                align: "left",
                cell: (listing) => listing.exchange ?? listing.mic,
              },
              {
                key: "currency",
                sort: (listing) => listing.currency,
                header: t`Currency`,
                align: "left",
                cell: (listing) => listing.currency ?? "—",
              },
              {
                key: "price",
                sort: (listing) => toNumber(listing.last_close),
                header: t`Last price`,
                cell: (listing) =>
                  listing.last_close && listing.currency ? (
                    <Money value={listing.last_close} currency={listing.currency} />
                  ) : (
                    "—"
                  ),
              },
              {
                key: "acts",
                align: "left",
                className: "acts",
                cell: (listing) => (
                  <button
                    className="iconbtn iconbtn--sm"
                    disabled={isCurrent(listing) || choose.isPending || !listing.currency || !listing.symbol}
                    title={
                      listing.currency
                        ? undefined
                        : t`Venue not checked — refresh the list to learn its currency`
                    }
                    onClick={() => choose.mutate(listing)}
                  >
                    <Trans>Choose</Trans>
                  </button>
                ),
              },
            ]}
          />
        )}

        <Buttons>
          <button
            className="iconbtn iconbtn--sm"
            disabled={refresh.isPending}
            onClick={() => refresh.mutate()}
          >
            {refresh.isPending ? t`Refreshing…` : t`Ask the directory again`}
          </button>
          {dead > 0 && (
            <span className="sub">
              <Plural
                value={dead}
                one="# venue hidden: the instrument is listed there but returns no prices"
                other="# venues hidden: the instrument is listed there but returns no prices"
              />
            </span>
          )}
        </Buttons>
      </section>
    </Modal>
  );
}
