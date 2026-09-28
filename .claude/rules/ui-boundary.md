---
paths:
  - "app/src-tauri/**"
  - "app/src/lib/api/**"
  - "app/src/lib/types/**"
---

# UI boundary (sq-app → sq-core, one-way)

See also `scope.md` (data-source lens), `plugins.md`, `ai-assistant.md`.

## Boundary
- `grep -r tauri core/` stays empty; `core/Cargo.toml` never gains `tauri`, `ts-rs`, `specta`. Host errors go through `UiError` (orphan rule).
- A command = parse args, take the lock, **one** core call, map the error. No SQL, no `Decimal` arithmetic in `app/`. A new UI number = a `calc/` function + test in `core/tests/`, then a one-line command.
- Wire field names are snake_case from core serde, pinned by `app/src-tauri/tests/wire_format/` (module per subject).
- Money crosses IPC as a string (`rust_decimal::serde::str`); `lib/format.ts` formats the string.
- Files cross as bytes (`&[u8]`), never paths — except the DB path from `app_data_dir()`.
- **Text never crosses IPC** (ADR-0023): host sends code + params; frontend writes the sentence (`ScopeOption` names, refresh `code`/`subject`, `ImportProblem` `code`/`params`, `UiError` `code`). A sentence the host writes into a file arrives as an argument (`report_save` takes the disclaimer from `lib/legal.ts`); canonical exports carry none (ADR-0078).
- Commands hand inputs, not tiles: `dashboard_summary` = valuation + scope's accounts + flat per-account cash; the frontend sums/labels. A new tile needs no new field.

## State, profiles, locking
- A profile is a folder `profiles/<id>/`; everything the host writes sits beside its DB. `AppState::db_path()` is the only path; changes only in `AppState::open_profile`. Background jobs copy it once and finish in their starting profile. Frontend reloads after `profile_open` (ADR-0047).
- A password profile opens **locked**: `AppState::store()` refuses with `locked` until `profile_unlock` (new commands gated for free by taking the store). DB is SQLCipher under a vault-sealed key (ADR-0049). Background threads never `Store::open` themselves: take `AppState::db_access()` (path + key) and hold `db_in_use()` while connected — `dbfile.rs` conversion must be the only connection.
- `Store` is `Send` not `Sync` → `Mutex<Store>`. Never hold the lock across network; background jobs open their own `Store` in a thread.

## Settings and mobile
- Preference location follows who acts on it: `AppSettings` = what the host reasons about (scope, `language` `"system"`|tag; OS locale via `AppStatus::system_locale`). `AppSettings::ui` is opaque `serde_json::Value` saved by `ui_state_save` — new widgets need no Rust change. `UiState::theme` lives there (`lib/theme.ts`, `data-theme`).
- Mobile-first: entry in `lib.rs` behind `mobile_entry_point`; base CSS is narrow, `@media (min-width: 700px)` for wide; dense tables get a card presenter; window `minWidth` 380.
