# 6: Keep market providers narrow and synchronous


- Status: Accepted

## Context

Quote retrieval, instrument search, listing discovery, caching, retries, and
persistence have different responsibilities. Combining them in each provider
would duplicate policy and make offline tests depend on network behavior.

## Decision

Define small synchronous traits for quotes, security search, and listing
directories. Each provider implements only its source-specific request and
parsing logic. `MarketDataService` owns provider registries, retry policy,
candidate orchestration, and storage updates.

Providers receive domain objects and inclusive date ranges, and return only
observations that the source actually supplied. Backward fill and missing-data
semantics remain in lookup and calculation layers.

## Consequences

New sources can be added without changing storage or calculations. Tests can
use deterministic fakes, while the desktop host may run the synchronous calls
in a worker when needed.
