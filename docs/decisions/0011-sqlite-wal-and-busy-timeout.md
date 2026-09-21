# 11: WAL mode and busy_timeout on every connection


- Status: Accepted

## Context

`Store` normally owns the only connection, but a background quote/FX refresh
opens a second one on its own thread so a network call (which can take
minutes) never holds the UI's connection hostage. Two connections to one
SQLite file need a concurrency policy.

## Decision

`Store::init` sets three pragmas on every connection:

- `foreign_keys = ON` — off by default in SQLite for historical compatibility;
  without it, `ON DELETE CASCADE` and `REFERENCES` in the schema are decorative.
- `journal_mode = WAL` — the default rollback-journal mode blocks all readers
  while a writer is active; WAL lets readers keep working during a write.
  Ignored on an in-memory database, where a second connection can't exist anyway.
- `busy_timeout = 5000` — WAL's other half: without it, a second writer gets
  `SQLITE_BUSY` immediately instead of waiting briefly for the first to finish.
