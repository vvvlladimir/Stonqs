---
paths:
  - "app/src-tauri/src/commands/**"
  - "app/src-tauri/src/scope.rs"
  - "app/src-tauri/src/state.rs"
  - "app/src-tauri/src/ai/tools/**"
  - "core/src/calc/scope.rs"
  - "core/src/calc/engine/**"
  - "core/src/calc/goals.rs"
  - "core/src/calc/limits.rs"
  - "core/src/calc/plans.rs"
  - "core/src/calc/alerts.rs"
  - "core/src/calc/watchlist.rs"
  - "core/src/calc/trades.rs"
  - "app/src/screens/Watchlist/**"
  - "app/src/screens/Positions/**"
  - "app/src/screens/Alerts/**"
  - "app/src/components/domain/positionColumns*"
---

# The scope (data-source lens)

- Host state `AppState::scope` (in `settings.json`). Reporting commands take `AppState::scope_selection()`; commands that *edit* or list accounts take `portfolio()` (narrowing there = data loss).
- Reporting commands take optional `source: Option<DataScope>` via `AppState::scope_selection_in` so a tile can have its own scope (ADR-0030). Not `app_status`/`period_ranges`. Report *exports* pass `None`.
- Facts about an *instrument* read the whole portfolio (`ScopeSelection::whole_portfolio`, guarded by `is_whole`) — e.g. the dividend block of `positions_at`. Value/result/weight stay scoped.
- Scope rewrites legs, not the list: `calc::scoped_transactions` turns an out-of-scope leg into an external flow (buy seen depot-only → `DELIVERY_INBOUND`; deposit-only → `WITHDRAWAL`). Depot-only = price performance only.
- `Transaction::scoped_from` (`#[serde(skip)]`, set only by `as_delivery`) lets `trading_volume` still count lens-made deliveries (fees stay in cost basis). Genuine deliveries count nothing; `as_external_cash` sets nothing (ADR-0044).

## Not scoped
- **Plans** (ADR-0033): `plans_list`, `plan_projection` take `portfolio()`. `plan_due` returns drafts; `plan_commit` writes rows + `plan_executions` in one command.
- **Goals / limits** (ADR-0068): a goal carries its accounts (`goal_accounts`, empty = whole); `PortfolioAnalytics::goal_progress` re-scopes. `goals_list`/`limits_list` take `portfolio()` + date, no `source`. Contribution = deposit or **unpaired** transfer leg (`paired_links`). Limits are measured, never enforced. No jurisdiction ships.
- **Alerts / events** (ADR-0034): host never notifies — `alerts_take_notifications` checks and marks in one call; `AlertNotifier` writes text and calls `notify` on start, after refresh, on `alerts` change. Unseen crossings → `tab-dot`; opening the log calls `alerts_mark_seen`, invalidates only `keys.alertsUnseen()`. `dev_alert_simulate` (DEV only) moves a real quote. Reported splits are only *offered* as corporate actions.
- **Watchlists** (ADR-0035): `watchlist_rows` reads quotes in quote currency (`calc::instrument_move`), no source. Position figures are joined on the frontend from `usePositions`/`usePositionReturns`. Shared catalogue `components/domain/positionColumns.ts` + one `ColumnPicker`; `UiState::position_columns`/`watch_columns` — **stored array is display order**, never re-sorted; sorts in `position_sort`/`watch_sort`. Purchase figures only under a named method (`cost-fifo`/`cost-average`); only the non-portfolio method costs a second pass (`needsCostQuery`); old ids via `resolveColumnIds`.
