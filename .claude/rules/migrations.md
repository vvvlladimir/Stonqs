# Migrations

The library is SQLCipher (`bundled-sqlcipher`); a plain file opens as with SQLite, an encrypted
one only through `Store::open_encrypted` (ADR-0049). A migration is plain SQL either way.

`storage/migrate.rs` holds `MIGRATIONS: &[(version, name, include_str!(...))]`, applied in order, each in its own transaction. `PRAGMA foreign_keys = ON` is set on every connection. **An applied migration is never edited** — add `000N_*.sql` and a new tuple.

Latest is `0025_price_index.sql`: `price_index (region, month, value, source)` — one consumer-price
level per region and month, the month stored as its first day so lexicographic order stays
chronological, and `source` recorded because index bases differ between publishers (2015=100 vs
2010=100), so a ratio is only meaningful inside one series. `index_coverage` mirrors
`quote_coverage` with one row per region — a region's series comes from exactly one source at a
time. `portfolios.inflation_region` is `NULL` for every existing portfolio, which is the state
"inflation is not reported at all" (ADR-0060).

Before that, `0024_source_usage.sql`: `source_usage (source, day, requests)` — requests spent per
source and UTC day, counted by `market::Budgets` before each call so a keyed source's daily
allowance survives a restart (ADR-0055).

Before that, `0023_market_sources.sql`: `fx_rates.source` (existing rows `ecb`) and
`security_symbols (security_id, source, symbol)`, cascading — an instrument's ticker at sources
*other* than its own `data_source`; the own source keeps reading `data_symbol`/`symbol`
(`Store::symbol_at`). A fallback writes with `INSERT OR IGNORE` (`fill_quotes`,
`save_fx_rates_from(.., false)`) so it never rewrites the primary's rows; a listing change clears
the table with the quotes. See ADR-0051/0052.

Before that, `0022_ai_usage.sql`: `ai_usage (id, chat_id, provider, model, input_tokens,
cached_tokens, output_tokens, reasoning_tokens, created_at)` — one row per **request**, because a
request is what the provider bills and one answer that calls tools is several. `chat_id` is
nullable with `ON DELETE SET NULL`, not cascading: the dashboard brief has no chat at all, and
deleting a conversation must not un-spend what it spent. A figure a provider does not report is
stored as 0. See ADR-0041.

Before that, `0021_ai_chat_effort.sql`: `ai_chats.effort` (`LOW | MEDIUM | HIGH`, existing rows
`MEDIUM`), beside `tool_mode` and for the same reason — switched from inside the conversation,
not an app setting. `ai_chats.model` gains a second job in the same change: it was a record of
what a chat was *started* with and is now also what it is *answered* with, so a model picked
mid-chat sticks. See ADR-0037.

Before that, `0020_ai_chat_tool_mode.sql`: `ai_chats.tool_mode` (`ASK | AUTO`, existing rows `ASK`).
`AUTO` runs read tools without asking *in that one chat* — it sits on the chat, not in settings,
the way a permission mode belongs to a Claude Code session: a chat opened permissively must not
make the next one permissive. Anything unrecognised reads back as `ASK`. See ADR-0037.

Before that, `0019_ai_grants.sql`: `ai_grants (chat_id, tool, created_at)`, cascading. It holds the
**session** scope only — "allow once" is by definition not remembered, so a `scope` column would
describe a value the table never has, and a global grant does not exist. There is deliberately no
`ai_tool_calls` table: a call and its result are already two blocks of the turns they belong to.
See ADR-0037.

Before that, `0018_ai_assistant.sql`: `ai_chats (id, title, provider, model, created_at, updated_at)`
and `ai_messages (id, chat_id, role, content, created_at)`, cascading, read in rowid order — two
turns of one exchange can share a timestamp but never a rowid. `content` is the host's JSON of
neutral `Block`s and is opaque to the core; `role` is `user | model`. No key is stored here, ever.
See ADR-0037.

Before that, `0017_watchlists.sql`: `watchlists (id, name)`, read in rowid order, and
`watchlist_items (watchlist_id, security_id, position)`, both cascading — deleting an instrument
takes it off every list. A save replaces a list's items. See ADR-0035.

Before that, `0016_alert_direction.sql`: `security_alerts.direction` (`UP | DOWN | BOTH`, existing rows
`BOTH`). The bookmark still follows every change of side; only crossings in that direction are
logged. See ADR-0034.

Earlier, `0015_alert_crossings.sql`: price alerts become one `PRICE` level crossed either way
(old `PRICE_ABOVE`/`PRICE_BELOW` rows are converted), `acknowledged_for`/`notified_for` are dropped,
and the rule gains its check bookmark (`side`, `checked_through`). `alert_crossings` is the log —
level and close copied at the time, `seen` for the navigation dot, `notified` for the OS
notification. See ADR-0034.

Before that, `0014_security_alerts.sql`: `security_alerts` and `security_events`. `security_events` holds the user's notes (`source IS NULL`) and
the dividends and splits a provider reported, upserted on a partial unique index over
`(security_id, kind, date) WHERE source IS NOT NULL`. `event_coverage` mirrors `quote_coverage`, kept
apart so quotes fetched before it existed backfill their events once. See ADR-0034.

Then `0013_investment_plans.sql`: `investment_plans` (schedule, amount, account, flat costs)
plus `plan_legs` (any number of instruments with weights; none means a cash contribution plan) and
`plan_executions(plan_id, occurrence_date, transaction_id)`. `transactions` gains no column: "when
was this plan last executed" is derived from the link table, so deleting the transaction offers the
occurrence again by itself. See ADR-0033.

And `0012_security_attributes.sql`: `securities.note` and `securities.wkn` plus the user's own attributes — `security_attribute_defs` (name, kind, unit, order) and `security_attributes` (value as `TEXT`, keyed by attribute id so a rename keeps the values). Kinds are `TEXT | NUMBER | DATE` and a kind never changes once the attribute exists; see ADR-0031.

Earlier still, in order:
- `0011_cash_and_exclusions.sql` — `cash_classifications` (so an account balance classifies like a security) and `taxonomy_exclusions` (per-tree off switch; see `taxonomy-and-rebalance.md` for why cash needs its own table and why `subject_id` carries no foreign key).
- `0010_default_taxonomies.sql` — data, not schema: three ready trees (Asset class / Region / Sector) with fixed ids; their names are user data, seeded in English and renamed by the user, never translated by the UI, editable/deletable; runs once.
- `0009_taxonomy_node_color.sql` — `taxonomy_nodes.color`: a palette slot (1–8), not a HEX; `NULL` means derive from node order.
- `0008_security_mic.sql` — `securities.mic` (ISO 10383 code of the chosen listing); venue name is derived via `market::mic`, never stored.
- `0007_deposit_accounts.sql` — deposit/securities account split, account groups; converts old `BROKERAGE` accounts by creating a `cash-<id>` deposit account and moving cash ops onto it.