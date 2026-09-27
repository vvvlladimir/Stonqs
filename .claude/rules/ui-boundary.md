# UI boundary (sq-app -> sq-core, one-way)

The dependency is one-way, enforced for free by the orphan rule (`Serialize` can't be implemented for `sq_core::Error` from the host — hence `UiError`).

Two neighbours carry what grew out of this file: `.claude/rules/ai-assistant.md` (everything under
`app/src-tauri/src/ai/`) and `.claude/rules/frontend.md` (the layers inside `app/src`).

- `grep -r tauri core/` must stay empty. `core/Cargo.toml` never gains `tauri`, `ts-rs` or `specta`, not even behind a feature.
- A command is: parse arguments, take the lock, one core call, map the error. No SQL and no `Decimal` arithmetic in `app/`. A new number for the UI is a function in `calc/` with a test in `core/tests/`, then a one-line command.
- Wire field names are snake_case, decided by the core's serde — not the frontend's habits. Pinned by `app/src-tauri/tests/wire_format/`, one module per subject.
- Money crosses IPC as a string (`rust_decimal::serde::str`), never a JS `number` for arithmetic. `app/src/lib/format.ts` rounds/groups the string directly.
- Files cross as bytes, not paths (`parse_csv` takes `&[u8]`; mobile pickers return a content URI, not a path). Exception: the DB path, from `app_data_dir()`, into `Store::open`.
- A profile is a folder (`profiles/<id>/`) and everything the host writes sits beside its
  database, so `AppState::db_path()` is the only path a command needs — and it changes only in
  `AppState::open_profile`. A background job copies it once and finishes into the profile it began
  in. After `profile_open` the frontend reloads the window. See ADR-0047.
- A profile with a password opens **locked**: `AppState::store()` refuses with `locked` until
  `profile_unlock`, so a new command is gated for free as long as it takes the store. Its
  database is SQLCipher-encrypted under a key sealed in the vault (ADR-0049) and is not open at
  all while locked. A background thread never calls `Store::open` itself: it takes
  `AppState::db_access()` (path + key) and holds `db_in_use()` for as long as its connection is
  open, because converting the file (`dbfile.rs`) must be the only connection.
- A plugin is **not** profile data: `AppState::plugins` reads `plugins/<id>/` beside the
  profiles folder, so a theme survives switching profile (ADR-0070). Its commands take no store,
  which is why a locked profile still has its colours; what a plugin *stores* belongs to the
  profile, in that profile's vault under the plugin's id. The host copies the manifest and the
  files it names and nothing else, and refuses a file name that leaves the package. It copies into
  a `.staging-*` folder and swaps it in by renaming, so a failed reinstall leaves the old version;
  a folder whose name is not its manifest's id is `Status::Misplaced` and offers nothing
  (`Plugins::loaded` is the one lookup by id). `Plugins::list` is read once and kept: install,
  remove and `plugins_list` (the Settings list) drop it, so one import sees one set of plugins from
  preview to commit, and a package edited on disk by hand appears when the list is opened.
- A plugin's **file reader** is the one piece of a stranger's *code* this host runs, and
  `plugins/sandbox.rs` is the whole of what it is granted (ADR-0086): a WASM component with no
  filesystem, no reachable address, a frozen clock and a seeded generator — linked at all only
  because a guest carrying a language runtime will not instantiate without them — under a memory
  ceiling and an epoch deadline. The epoch is the engine's, shared by every store, so one ticker
  thread moves it while anything runs and a deadline is a number of ticks — never a bump of its
  own, which would end every other plugin's call too. A module is compiled once per version of its file (`sandbox::component`,
  keyed by path, size and time, dropped by `sandbox::forget` on install and remove), and every
  command that can run one — `import_load`, `transactions_export_save`, `plugin_install` — is
  `async` and goes through `commands::off_thread`: a synchronous Tauri command runs on the main
  thread, and a module is allowed seconds. It runs **once**, in `import_load`, and what it produced replaces
  the bytes in `AppState::import_file`, so every later preview and the commit read a
  `stonqs.transactions` document through `parse_canonical`. Preview and commit therefore cannot
  diverge, and `core` gains no dependency: the registry is `Plugins::read_file`, one layer above
  `import::parse_file`, asked after the two self-describing shipped formats and before the CSV
  reader, which accepts nearly anything. `not-mine` moves on, and so does a module that broke
  (trap, deadline, memory — `Refusal::Broken`), named in `skipped_readers`; only the reader's own
  `malformed` is `UiError::Reader`, never a fall-through (ADR-0087). `needs-password` is
  `UiError::FileProtected { reader, tried }`; the wizard asks, and `import_load` is called again with
  `unlock`, whose password reaches the reader that asked and no other, and is never stored. The row schema is **not**
  restated in WIT — the document carries its own `format` and `version` (ADR-0066).
- A plugin's **dashboard widget** (ADR-0083) is the one piece of a stranger's *JavaScript* the app
  runs. The host serves it from its own scheme (`stonqs-plugin`, `commands::plugins::page`)
  as one page — `plugins/widget_shim.js` plus the module inline — under a CSP of its own with no
  `connect-src` and a nonce per response. What a CSP does not cover the page closes itself: the
  shim deletes every `RTC*` global before the module runs (a STUN server is the page's to choose)
  and DNS prefetching is off; the frame is `sandbox="allow-scripts"`, so it has no
  origin, and Tauri injects IPC into the main frame only. Nothing is asked *for*: the manifest's
  `reads` (a closed list) is what the host builds — `plugins::reads::project`, through
  `plugin_reads`, one query per frame (`usePluginReads`) under the tile's scope and period — in the
  bridge's own field names, versioned by `api`; `lib/pluginBridge.ts` types what arrives and posts
  it. An assistant tool of the same package is handed the same function's answer (ADR-0088). The tile's header names the plugin, because what it shows is the plugin's number
  (ADR-0082).
- A plugin's **screen** (ADR-0084) is the same page under `/screen/<plugin>/<id>` beside
  `/widget/…`, following the app's lenses rather than a source of its own — which is why
  `transactions` is a screen's read only (`transactions_list` has no `source`). It may keep **one
  document** in the profile (`plugin_state`, migration 0031) if its manifest says `storage`:
  `plugin_state_get/save` refuse anybody else, cap it at 256 KiB, and ask nothing, because it is
  the plugin's data and reaches no operation, account or figure. The frame's `save` message is the
  bridge's only write.
- A plugin's **file writer** is the reader turned round (ADR-0080): `wit/writer.wit`,
  `write(canonical) -> result<bytes, reason>`, run in the same `plugins/sandbox.rs`. It is handed
  what `transactions_export` already built — the screen's filter, the store released first — and
  never sees the portfolio or a path; `transactions_export_save` writes what it returns. Its
  expectation is compared **byte for byte**, because its output is bytes another program judges.
- `Store` is `Send`, not `Sync`, hence `Mutex<Store>`. Never hold that lock across a network call — background jobs open their own `Store` on `AppState::db_path` in a separate thread.
- **Text never crosses IPC.** The host and the core send a code, a key and the values behind it;
  the sentence is written in the frontend, where the language is known — see ADR-0023. `ScopeOption`
  carries names (`components/domain/scopeLabel.ts` composes), a refresh failure carries `code`/`subject`
  (`MarketRefresh.headline`), `ImportProblem` carries `code`/`params` (`Import/labels.problemDetail`),
  `UiError` carries `code` (`Async.useErrorText`). Every string left in Rust is English and is a
  developer detail, never a headline. A sentence the *host* must write into a file therefore
  arrives as an argument: `report_save` takes the disclaimer footer (`lib/legal.ts`) rather than
  spelling it, and a canonical export carries none at all — that file is read back by the import
  (ADR-0078).
- Commands hand the UI *inputs*, not tiles: `dashboard_summary` returns valuation + scope's accounts + a flat per-account cash list; the frontend sums/labels. A new tile must not require a new field here.
- Mobile-first is structural: entry point lives in `lib.rs` behind `mobile_entry_point`, base CSS is the narrow layout (`@media (min-width: 700px)` for wide), every dense table ships with a card presenter (`components/ui/DataTable.tsx` derives one from the columns). Window `minWidth` is 380 so compact layout can be tested by resizing.
- Where a preference lives follows who acts on it. `AppSettings` holds what the *host* must reason
  about — the scope, and `AppSettings::language` (`"system"` or a locale tag; the OS language arrives as
  `AppStatus::system_locale`, read with `sys-locale`). `AppSettings::ui` is an opaque
  `serde_json::Value` the host stores and never parses, with its own command (`ui_state_save`), so a
  new widget is not a Rust change; `UiState::theme` sits there because only the frontend acts on it
  (`lib/theme.ts` sets one `data-theme` attribute, `useTheme` follows the OS while the preference is
  `"system"`).

## The scope (data-source lens)

- The selector is host state (`AppState::scope`, persisted in `settings.json`). Reporting commands take `AppState::scope_selection()`; commands that *edit* the portfolio or list accounts take `portfolio()` — narrowing there would be data loss.
- A reporting command also takes an optional `source: Option<DataScope>` and resolves it through
  `AppState::scope_selection_in`: the picker is the app's lens and stays the default, but one
  request may name a scope of its own so a dashboard can hold a tile per account without the
  screen around it changing (ADR-0030). `app_status` and `period_ranges` do not — the status is
  the app's own and the period axis is the board's, shared by every tile on it. A report
  *export* passes `None`: it is an export of what the screen shows.
- A fact about an *instrument* is read from the whole portfolio, not from the scope
  (`ScopeSelection::whole_portfolio`, guarded by `is_whole` so the extra holdings pass is paid
  only by a narrowed scope). The dividend block of `positions_at` is the case: a payment settles
  on the deposit account, so a depot-only scope drops it and a schedule read from that lens would
  call a quarterly payer "never paid". Value, result and weight in the same row stay the scope's.
- A scope narrows the point of view, not the transaction list. Every transaction has two legs (securities on `account_id`, cash on `settlement_account_id`); `calc::scoped_transactions` rewrites whichever leg falls outside scope into an external flow (buy seen depot-only → `DELIVERY_INBOUND`; deposit-only → `WITHDRAWAL`). A depot-alone scope is therefore price-performance only — dividends/interest/fees land on the deposit account.
- That rewrite records what it rewrote: `Transaction::scoped_from` (`#[serde(skip)]`, set only by
  `as_delivery`) is how `trading_volume` still counts a trade the lens turned into a delivery. It
  must, because `as_delivery` keeps `fees` — a buy commission is part of the lot's cost basis — so
  excluding it left a fee rate with no trades to have caused it. A *genuine* delivery still counts
  for nothing, and the cash-side rewrite (`as_external_cash`) sets nothing: from a deposit account
  the trade happened elsewhere. See ADR-0044.
- An investment plan is *not* scoped: it is an intention about the portfolio, so `plans_list` and
  `plan_projection` take `portfolio()` — narrowing the picker must not hide next month's savings.
  A plan writes nothing on its own: `plan_due` returns the drafts, and `plan_commit` is one command
  (rows + `plan_executions` link) so a half-written occurrence cannot leave the plan offering a
  month whose purchases are already in the ledger. See ADR-0033.
- A goal and a contribution limit are *not* scoped either, and for the plan's reason (ADR-0068): a
  goal **carries its own accounts** (`goal_accounts`, empty = the whole portfolio) and
  `PortfolioAnalytics::goal_progress` re-scopes to them, so the picker changes nothing; a limit
  belongs to one account. `goals_list` / `limits_list` take `portfolio()` and a date, never a
  `source`. A contribution is a deposit or an **unpaired** transfer leg — the same `paired_links`
  reading TWR takes — so an internal move eats no allowance, and a limit is **measured, never
  enforced**: no command refuses a transaction over it. No jurisdiction ships in the binary; the
  amount, the currency and `MM-DD` of the year's opening are the user's.
- Alerts and instrument events are *not* scoped either: a level is about an instrument. The host
  never sends a notification — `alerts_take_notifications` checks the rules and hands over the
  unannounced crossings, marked in the same call, and `AlertNotifier` writes the text and calls
  `notify` (`lib/api.ts`) on start, after each refresh and on every `alerts` change, because the
  startup refresh can end before a window listens. Unseen crossings put `tab-dot` on the Alerts nav
  item; opening the Log popup calls `alerts_mark_seen` and invalidates only `keys.alertsUnseen()`,
  so the lines in it keep their dot while it is open. The debug-only simulator
  (`dev_alert_simulate`, `import.meta.env.DEV`) moves a real quote, never a crossing. A reported
  split is only *offered* as a corporate action. See ADR-0034.
- A watchlist is *not* scoped: `watchlist_rows` reads each instrument's own quotes in its quote
  currency (`calc::instrument_move`) and takes no source. What a *position* in that instrument
  shows is joined on the frontend from `usePositions`/`usePositionReturns` under the picker — never
  recomputed for the list. Both tables draw from one column catalogue
  (`components/domain/positionColumns.tsx`) and one `ColumnPicker`; the choices are
  `UiState::position_columns` and `watch_columns` — **the stored array is the display order**, so
  the picker returns it as dragged and never re-sorts it into catalogue order. How each table is
  ordered is stored beside it (`position_sort`, `watch_sort`). A purchase figure is only ever
  offered under a named method (`cost-fifo` / `cost-average`, …): the portfolio's own method is
  already in the position row, so only the other one costs `positions_cost_basis` a second
  holdings pass (`needsCostQuery`), and a choice stored before the naming resolves through
  `resolveColumnIds`. See ADR-0035.
