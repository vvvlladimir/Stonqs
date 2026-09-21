# 51: An FX rate comes from the first source that publishes the pair

- Status: Accepted

## Context

Every rate came from the ECB. The ECB publishes about thirty currencies and suspended the rouble on
2022-03-01, so a portfolio holding anything outside that list — RUB, ARS, VND, NGN, most of the
world — could never be valued in its base currency: the refresh failed for the pair every time and
every figure behind it showed as missing. An ECB outage failed every pair at once.

ADR-0050 made sources a catalogue; this is the first role that has more than one of them.

## Decision

`FxService` holds an ordered chain: `DEFAULT_FX` (ECB) first, then the other FX-capable rows in
catalogue order — today Yahoo, whose `BASEQUOTE=X` series covers practically every currency.

- A source declares what it publishes (`FxProvider::covers`, default: everything). One that does
  not publish either currency of the pair is skipped without a request. The ECB lists what it
  still publishes.
- An **error** from a source falls through to the next one. An **empty** answer is final: a
  weekend or a holiday legitimately returns nothing, and asking on would pull a second source into
  a series for no reason.
- `ensure_rates` returns the rows saved and the id of the source that answered.
  `ensure_rates_from` keeps asking one named source for callers that must.
- The Yahoo adapter rejects a series quoted in any currency but the pair's quote currency.
- Cross-rates are still synthesized only inside one provider; the chain never combines two.

## Alternatives

- **A central bank per currency (CBR for RUB, BCRA for ARS, …).** Official fixings, but one adapter
  per country and still a gap for every country not covered. Kept for later, per currency, only
  where the market close is not good enough.
- **Fall through on empty too.** More rows, but a series stitched from two sources on ordinary
  weekends.
- **Frankfurter as the fallback.** It republishes ECB data, so it adds resilience against an ECB
  outage but no currencies.

## Consequences

- Any currency Yahoo quotes is now valued; the refresh no longer fails permanently for RUB & co.
- During an ECB outage the market close fills in. The two differ by a fraction of a percent; a
  trade's own rate is fixed on the transaction and is not affected.
- `fx_rates` does not yet record which source wrote a row; a source column arrives with the
  per-source storage work, together with the rule that a fallback never overwrites the primary.
