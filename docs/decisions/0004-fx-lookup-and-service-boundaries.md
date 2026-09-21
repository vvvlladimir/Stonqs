# 4: FX lookup and service boundaries


- Status: Accepted

## Context

Portfolio calculations need historical rates, deterministic tests, and a
single interpretation of missing dates. Providers should not own caching or
database policy.

## Decision

Use one-method providers that return a base/quote series for a date range.
`RateLookup` handles identity, inverse pairs, and backward fill ("as of, or
last known before"). Cross-rates are synthesized only by the ECB provider,
where both legs share one source and date; storage and cache lookup never
invent cross-rates.

`FxService` owns provider selection, retry policy, and persistence. The
in-memory cache mirrors lookup semantics and uses ordered date series for
efficient backward lookup.

## Consequences

Providers remain replaceable and easy to test. Database and in-memory
calculations agree, and missing market data cannot silently become zero.
