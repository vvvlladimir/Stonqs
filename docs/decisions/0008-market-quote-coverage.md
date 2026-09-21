# 8: Track quote coverage separately from returned quotes


- Status: Accepted

## Context

Providers omit weekends, holidays, and other unavailable observations. Using
the presence of rows to decide whether a range was fetched would repeatedly
request dates for which no quote can exist.

## Decision

Persist requested date coverage separately from quote rows. `MarketDataService`
fetches only uncovered head and tail ranges and extends coverage even when a
provider returns no rows. A caller supplies `settled_through`; unsettled days
are fetched but are not marked covered so an intraday refresh can obtain the
final close.

The in-memory `PriceCache` mirrors store lookup semantics with ordered series.
As-of lookup backward-fills, while `price_before` steps to the preceding quote
rather than subtracting one calendar day.

## Consequences

Refreshes are bounded and empty provider responses are not retried forever.
Calculations remain deterministic and consistent between SQLite and memory.
