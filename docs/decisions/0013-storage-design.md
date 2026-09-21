# 13: SQLite storage design

- Status: Accepted

## Context

Stonqs is a single-user desktop application with one local database. The
storage layer must remain easy to inspect, portable across supported platforms,
and usable in tests without a running service.

## Decision

Use one `rusqlite` connection owned by `Store`, with repository methods split
across storage modules. Use SQLite directly instead of an ORM or an async
database abstraction. Build SQLite into the application so users do not need a
system SQLite library.

Enable foreign keys, WAL journaling, and a busy timeout for every connection.
Store `Decimal` values and ISO dates as `TEXT`; conversion is centralized in
`storage/mod.rs`. Apply embedded SQL migrations in order, each in its own
transaction. An applied migration is immutable; schema changes require a new
migration file.

## Consequences

The implementation has one source of truth and no storage mock trait. Tests use
the same schema through an in-memory SQLite database. Repository methods must
preserve transactional writes and explicit validation at the storage boundary.
