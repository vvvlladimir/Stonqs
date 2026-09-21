# 36: The quote source names the venue it quotes

- Status: Accepted

## Context

A venue reached the instrument one way only: OpenFIGI maps an ISIN to the tickers it trades under,
the user picks one, and `security_set_listing` writes the MIC. An instrument with no ISIN therefore
had no venue and no way to get one — `security_listings` refused outright, and the picker showed
the refusal as an error.

That is most instruments added from a provider search. Yahoo returns no ISIN at all — neither the
search endpoint nor the chart profile carries one — so every instrument put on a watchlist arrived
with an empty ISIN and an empty venue, and nothing in the app could fill either. ADR-0022 left the
final say on a listing with the user, which is right, but it left the common case with nothing to
say it about.

The quote source already knows the answer. It is quoting one specific listing: the symbol carries
the venue as a suffix (`EUNL.DE`), and where it does not — the US venues share a bare ticker — the
response names the exchange (`fullExchangeName: "NYSE"`). Reading that is not guessing; it is
recording which listing the price series we store actually came from.

## Decision

`SecuritySearch` gains `mic_for(symbol, exchange)`, the inverse of `symbol_for`. Yahoo implements
it from its suffix table, falling back to a small list of its own exchange names for the venues
whose symbols carry no suffix. A suffix we do not know is an unsupported venue, not a US listing.

`SecurityMatch` and `SecurityDraft` carry the resulting `mic`, so every path that creates an
instrument from a provider answer records the venue with it: `security_save` (which the watchlist's
add dialog passes it through), `security_identify`, and the import's resolution. `security_identify`
no longer clears the MIC to `None` — it writes what the source named, because the symbol it is
replacing is the one the old venue belonged to.

`MarketDataService::listings_by_symbol` gives the picker something to show without an ISIN: the
search is run for the bare ticker and every match on the same ticker that the source can place on a
supported venue becomes a listing, probed like any other. `Listing::isin` is empty there, which is
also why that answer is not cached — the listings table is keyed by ISIN.

## Alternatives

**Find the ISIN first.** Nothing free maps a ticker to an ISIN: OpenFIGI's mapping response carries
FIGI, ticker and exchange code but no ISIN, and Yahoo has no ISIN field anywhere. This is not a
matter of another request.

**Probe every supported venue.** Build `symbol_for(ticker, mic)` for all 29 markets and ask the
provider which respond. It is 29 requests, and it cannot tell XNAS from XNYS from ARCX at all — all
three produce the bare ticker. The provider's own exchange name answers that in one request.

**Leave it to the user.** Offer the closed market list and let the user pick. It states nothing the
provider does not already know, and for a US ticker it asks the user to choose between three venues
that are indistinguishable from anything the app can show them.

## Consequences

An instrument added from a search now arrives with its venue set, so the "no venue chosen" state is
what it should be: rare, and either a manual-price instrument or one whose venue we genuinely do not
support. Existing venue-less instruments are not migrated — opening the venue picker fills them,
now that it works without an ISIN.

Choosing a listing whose provider symbol is already the security's is now possible — it is how a
venue-less instrument gets its MIC — so `security_set_listing` deletes the quotes only when that
symbol actually changes, and the venue picker calls a row "current" only when symbol and MIC both
match. A matching symbol alone is not a chosen listing.

A MIC is a fact about which series we store, not a promise about where the user traded. The
`Instruments` screen keeps a "No venue" slice over `mic IS NULL` so the remainder stays visible,
separate from the "Not identified" slice, which is the different fault of an ISIN in the ticker
field.

`mic_for` is per source, like `symbol_for`. A provider that spells venues differently implements its
own; one that says nothing returns `None` and its instruments stay venue-less, as before.
