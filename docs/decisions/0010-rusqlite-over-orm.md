# 10: `rusqlite` over an ORM


- Status: Accepted

## Context

The app is a single-user desktop app: one process, one writer, no concurrent
clients. `sqlx` and similar crates buy async support and compile-time query
checking against a live database — capabilities this app has no use for and
would pay a real cost for (an async runtime, a build-time DB dependency).

## Decision

Use `rusqlite`, a thin wrapper over the SQLite C library, with the `bundled`
feature so SQLite compiles into the binary and needs no system library.

Over an ORM (`diesel`, `SeaORM`): the SQL executed is visible at the call
site, which matters for a codebase meant to be readable by someone learning
Rust — an ORM's type errors tend to unroll across screens.

No trait abstraction sits above `Store`: there is exactly one implementation,
and SQLite starts up in memory fast enough that swapping it out in tests buys
nothing.
