# 9: Normalize Yahoo wire data at the provider boundary


- Status: Accepted

## Context

Yahoo exposes quotes, search results, and profiles through separate unofficial
endpoints. Prices arrive as floating-point JSON values, and minor currency
units such as `GBp` can be reported with prices in pence.

## Decision

Use the chart endpoint for daily quotes and profiles, and the search endpoint
only for candidate discovery. Round floating-point wire values to remove
single-precision noise before converting to `Decimal`. Normalize minor
currency units and scale the price at the same boundary, so downstream code
receives a consistent major-unit price/currency pair.

Use exchange-specific Yahoo suffixes only when constructing a symbol from a
MIC. Keep the Stooq adapter as an independent alternative implementation.

## Consequences

The rest of the core handles decimal money and paired currencies without
Yahoo-specific branches. Provider quirks remain isolated and are covered by
parser tests that do not require network access.
