# 62: An upgrade copies the database before it changes it

- Status: Accepted

## Context

Opening a database applies every pending migration (ADR-0012). Each one runs in its own
transaction, so a migration that fails part-way leaves nothing behind — but a *sequence* of
migrations that succeeds is not undoable. A version that drops a column, converts rows in place or
rewrites a table cannot give the old shape back, and neither can a migration that was correct in
its SQL and wrong in its intent.

Until now nothing stood between a user's ledger and a bug shipped in a migration. That is a
different class of risk from the rest of the application: a wrong number on a screen is a bug
report, a lost portfolio is a person's records of years of trading, kept nowhere else — the whole
point of the app is that the data never leaves their machine, which also means nobody else has a
copy.

The user is not in a position to prevent this. The upgrade happens while the app starts, before
any window is drawn, and the file lives in a directory they have no reason to have visited.

## Decision

Before the first migration of an **upgrade** runs, the database file is copied beside itself as
`<name>.bak-v<version>`, where the version is the schema it is being upgraded *from*.

- Only an upgrade is copied. A database with no applied migrations holds nothing to lose, so
  creating a profile does not leave an empty backup next to it.
- A WAL checkpoint runs first. Committed pages can still be sitting in the `-wal` file, and a byte
  copy taken without the checkpoint would be missing the most recent transactions — exactly the
  ones the user would notice.
- An encrypted database (ADR-0049) is copied as the encrypted bytes it already is. The backup is
  therefore no weaker than the original and needs no key of its own.
- A failed copy **aborts the upgrade** (`Error::Backup`). Refusing to start is recoverable; a
  migration applied over data with no copy is not.
- The three newest copies are kept and older ones deleted. A backup is a whole database file, so
  keeping one per version ever applied would eventually cost far more than what it protects.
- An in-memory database passes no path and is never copied.

This lives in `storage`, not in the host: every caller that opens a file — the app, the CLI, a
future one — is covered by the same code, and `Store::open` is the only place that knows both the
path and that migrations are about to run.

## Alternatives

- **Back up in the host, before opening the store.** The CLI would be unprotected, and the host
  would have to reason about the encrypted-file conversion path as well. The knowledge belongs
  where the path and the migration meet.
- **Offer restore in the interface.** A restore is a rare, deliberate act performed by someone who
  already knows something is wrong; a file named after the version it came from, sitting beside the
  database, is enough to hand to support or to copy back by hand. A restore button is a feature to
  design later, not a reason to ship no copy now.
- **Keep every backup.** Simpler, and unbounded: a 200 MB database upgraded through a year of
  releases would quietly fill a disk.
- **Copy on every open.** Most opens change nothing, so this would be a large write at every start
  for no gain.

## Consequences

- A profile directory can hold up to three extra database-sized files. They appear only after an
  upgrade, and they are plainly named.
- An upgrade now fails on a full or read-only disk where it previously proceeded. That is the
  intended trade.
- The copy is a plain file on disk. For an unencrypted profile it is exactly as readable as the
  database it came from — no better, no worse.
- Restoring is manual: quit the app, rename the copy over the database. This is the deliberate
  minimum, and it is documented where a user could be told to do it.
