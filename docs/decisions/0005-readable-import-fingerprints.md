# 5: Human-readable import fingerprints


- Status: Accepted

## Context

Re-importing a broker file must not duplicate transactions, but users also
need to understand why a row was classified as a duplicate. A cryptographic
hash is compact but opaque during support and debugging.

## Decision

Use a stable, delimited string containing account, date, kind, security,
quantity, amount, and currency. Normalize decimal spellings before comparison.
Fees and notes are excluded because brokers may change those fields between
exports without changing the underlying transaction.

## Consequences

The same operation produces the same identity in previews and storage. The
identity is longer than a hash, but it can be inspected directly and remains
adequate for the application's expected transaction volumes.
