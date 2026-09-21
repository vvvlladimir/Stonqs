# 12: Migrations as a plain array of SQL files


- Status: Accepted

## Context

The database is a single local file belonging to one user. Migration crates
(`refinery`, `sqlx-migrate`) solve problems this app doesn't have: multiple
environments, rollbacks, a team coordinating schema changes concurrently.

## Decision

`storage/migrate.rs` holds `MIGRATIONS: &[(version, name, sql)]`, each entry
pulled in via `include_str!`. This compiles the SQL into the binary — no
migrations directory to locate on the user's disk, and a missing/misnamed
file is a build error instead of a runtime one. The one rule that matters at
this scale: the list only grows, order is fixed, and an applied migration is
never edited (see `.claude/rules/migrations.md`).
