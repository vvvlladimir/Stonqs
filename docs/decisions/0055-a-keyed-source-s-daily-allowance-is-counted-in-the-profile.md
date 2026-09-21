# 55: A keyed source's daily allowance is counted in the profile

- Status: Accepted

## Context

Free plans of keyed sources are metered per day (Twelve Data 800 requests, EODHD 20). ADR-0053
left that to the breaker: a source answering 429 rests for the rest of the refresh. That spends
the allowance on requests that were bound to fail, and a restart forgets it entirely, so the next
refresh walks into the same wall — and some providers suspend a key that keeps hitting it.

## Decision

- `SourceInfo::per_day` is the free plan's allowance; a source without a published one has none.
- `market::Budgets` counts requests per source and UTC day in `source_usage` (migration 0024) —
  one row per source and day, so the count survives a restart and follows the profile.
- Both services spend from it before each call (`Budgets::spend`, one request per call whatever the
  retries); a spent allowance is `Error::RateLimited` without a request, so the chain moves on to
  the next source exactly as it would after a 429.

## Alternatives

- **Pace by minute as well.** The per-minute limits are what the breaker already absorbs; the
  per-day one is the one that runs out.
- **Read the remaining allowance from response headers.** Few providers send it, each in its own
  header, and a request is still spent to learn it.

## Consequences

- A heavy refresh against a keyed source stops at its allowance and the rest falls back or waits
  for the next day. The allowance is the free plan's; a paid plan is not known to the app.
