---
paths:
  - "core/migrations/**"
  - "core/src/storage/**"
---

# Migrations

- SQLCipher (`bundled-sqlcipher`); plain files open as SQLite, encrypted ones only via `Store::open_encrypted` (ADR-0049). Migrations are plain SQL.
- `storage/migrate.rs::MIGRATIONS: &[(version, name, include_str!(..))]`, applied in order, one transaction each. `PRAGMA foreign_keys = ON` on every connection.
- **Never edit an applied migration** — add `core/migrations/000N_*.sql` + a tuple, and add its line below.
- DB version above this build's latest → `Error::NewerDatabase`, nothing opened.
- Before the first migration of an upgrade: `migrate::back_up` copies to `<name>.bak-v<from>` (WAL checkpoint first; encrypted copied as-is; failure = `Error::Backup`, aborts). Not for a fresh DB or in-memory (`None` path). Keep the 3 newest (ADR-0062).

## History (newest first; details in the ADR)

- 0031 `plugin_state (plugin, state, updated_at)` — one opaque JSON doc per plugin with `storage`; in the DB so it's encrypted; survives plugin removal (ADR-0084).
- 0030 `security_symbols.latest` + partial unique index, ≤1 per instrument (ADR-0079).
- 0029 `contribution_limits.withdrawals_restore`, default 0 (ADR-0071).
- 0028 `goals` + `goal_accounts` (cascade; none = whole portfolio), `contribution_limits` (`year_starts_on` = `MM-DD`); `expected_return` is a fraction, user's assumption; nothing enforced (ADR-0068).
- 0027 `transactions.external_id`, nullable, indexed; identity checked before fingerprint; same id + different values replaces (ADR-0065).
- 0026 `transactions.fee_currency`/`tax_currency`, `NULL` = transaction currency; equal currency written as `NULL` (ADR-0064).
- 0025 `price_index (region, month, value, source)`, month = first day; `index_coverage` one row per region; `portfolios.inflation_region` NULL = not reported (ADR-0060).
- 0024 `source_usage (source, day, requests)` for `market::Budgets` (ADR-0055).
- 0023 `fx_rates.source` (old rows `ecb`); `security_symbols (security_id, source, symbol)` cascading — tickers at *other* sources; own source reads `data_symbol`/`symbol` (`Store::symbol_at`); fallbacks `INSERT OR IGNORE` (ADR-0051/0052).
- 0022 `ai_usage` — one row per request; `chat_id` nullable `ON DELETE SET NULL`; unreported figures = 0 (ADR-0041).
- 0021 `ai_chats.effort` (`LOW|MEDIUM|HIGH`, default `MEDIUM`); `ai_chats.model` is also what the chat is answered with (ADR-0037).
- 0020 `ai_chats.tool_mode` (`ASK|AUTO`, default/unknown `ASK`) (ADR-0037).
- 0019 `ai_grants (chat_id, tool, created_at)` cascading — session scope only; no `ai_tool_calls` table (ADR-0037).
- 0018 `ai_chats`, `ai_messages` (cascade, rowid order; `content` = host JSON of `Block`s, opaque to core; `role` `user|model`). Never a key (ADR-0037).
- 0017 `watchlists`, `watchlist_items (watchlist_id, security_id, position)` cascading; save replaces items (ADR-0035).
- 0016 `security_alerts.direction` (`UP|DOWN|BOTH`, default `BOTH`) (ADR-0034).
- 0015 price alerts → one `PRICE` level; drop `acknowledged_for`/`notified_for`; add `side`, `checked_through`; `alert_crossings` log with `seen`/`notified` (ADR-0034).
- 0014 `security_alerts`, `security_events` (notes: `source IS NULL`; provider events upserted on partial unique `(security_id, kind, date) WHERE source IS NOT NULL`), `event_coverage` (ADR-0034).
- 0013 `investment_plans`, `plan_legs` (none = cash plan), `plan_executions(plan_id, occurrence_date, transaction_id)`; "last executed" derived from links (ADR-0033).
- 0012 `securities.note`/`wkn`, `security_attribute_defs`, `security_attributes` (value `TEXT`, keyed by attribute id); kinds `TEXT|NUMBER|DATE`, immutable (ADR-0031).
- 0011 `cash_classifications`, `taxonomy_exclusions` (see `taxonomy-and-rebalance.md`).
- 0010 data: three default trees (Asset class / Region / Sector), fixed ids, English names = user data, never translated.
- 0009 `taxonomy_nodes.color` = palette slot 1–8, `NULL` = by order.
- 0008 `securities.mic` (ISO 10383); venue name derived via `market::mic`.
- 0007 deposit/securities split, account groups; old `BROKERAGE` → `cash-<id>` deposit account gets the cash ops.
