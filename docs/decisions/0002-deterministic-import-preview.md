# 2: Deterministic, user-overridable import preview


- Status: Accepted

## Context

Broker CSV formats are inconsistent, and automatic detection can be wrong in
ways that remain invisible until a report is produced. Network lookups also
make a preview non-reproducible.

## Decision

Keep import as a staged pipeline:

- parsing discovers file settings but keeps cell values as strings;
- mapping discovers columns and aliases, while preserving every choice for
  user overrides;
- preview builds drafts without writing to storage or resolving securities
  over the network;
- commit is the only stage that mutates `Store`.

Rows carry explicit statuses and problem severities. Warnings may be written,
but errors block a row. This lets the UI show every issue before any write.

## Consequences

The same input and explicit mapping produce the same preview. Saved mappings
can be reused for later exports from the same broker, and the commit boundary
prevents a partially imported file.
