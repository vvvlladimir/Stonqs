---
paths:
  - "app/src-tauri/src/plugins/**"
  - "app/src-tauri/wit/**"
  - "app/src-tauri/src/commands/plugins.rs"
  - "app/src-tauri/src/import_templates.rs"
  - "app/src/lib/pluginBridge.ts"
  - "app/src/lib/theme.ts"
  - "app/src/components/domain/PluginFrame.tsx"
  - "app/src/screens/PluginScreen/**"
  - "app/src/screens/dashboard/widgets/plugin.tsx"
---

# Plugins (ADR-0070)

## Package and registry
- Not profile data: `plugins/<id>/` beside the profiles folder (a theme survives profile switches); commands take no store. What a plugin *stores* is profile data.
- Install copies manifest + named files only (refuses names leaving the package) into `.staging-*`, then renames (failed reinstall keeps old). Folder ≠ manifest id → `Status::Misplaced`. `Plugins::loaded` is the one lookup by id. `Plugins::list` cached; dropped by install/remove/`plugins_list`.
- Content ids are `valid_id`.

## WASM sandbox (`plugins/sandbox.rs`, ADR-0086)
- No FS, no network, frozen clock, seeded RNG (linked only because runtimes need them), memory ceiling, epoch deadline. One shared ticker thread moves the engine epoch; a deadline is a tick count — never bump the epoch yourself.
- `sandbox::component` caches per (path, size, mtime); `sandbox::forget` on install/remove. Commands that run one (`import_load`, `transactions_export_save`, `plugin_install`) are `async` via `commands::off_thread`.
- **Reader** (`wit/reader.wit`): runs once in `import_load`; output replaces `AppState::import_file`, later read by `parse_canonical`. Registry = `Plugins::read_file`, asked after the two self-describing formats, before CSV. `not-mine` and `Refusal::Broken` (trap/deadline/memory) move on, listed in `skipped_readers`; only `malformed` is `UiError::Reader` (ADR-0087). `needs-password` → `UiError::FileProtected { reader, tried }`; retry with `unlock`, password goes only to that reader, never stored. Row schema isn't in WIT — the doc carries `format`/`version` (ADR-0066). Manifest needs `sample` **and** `expected`; failing install = nothing installed.
- **Writer** (`wit/writer.wit`, ADR-0080): `write(canonical) -> result<bytes, reason>`, fed what `transactions_export` built (store released first); never sees portfolio or paths. Expectation compared byte for byte.
- **Tool** (`wit/tool.wit`, `plugins/tool.rs`, ADR-0085): see `ai-assistant.md`.

## Frames: widget and screen
- Widget (ADR-0083): served from `stonqs-plugin` scheme (`commands::plugins::page`) as one page = `plugins/widget_shim.js` + module inline; own CSP, no `connect-src`, nonce per response. Shim deletes `RTC*` globals; DNS prefetch off; frame `sandbox="allow-scripts"` (no origin, no IPC).
- Data: manifest `reads` (closed list) → `plugins::reads::project` via `plugin_reads`, one query per frame (`usePluginReads`) under the tile's scope/period, in the **bridge's** field names, versioned by `api`; `lib/pluginBridge.ts` types it (ADR-0088). Tile header names the plugin (ADR-0082).
- Screen (ADR-0084): same page at `/screen/<plugin>/<id>`, follows the app's lenses; `transactions` read is screen-only. With `storage`: one doc via `plugin_state_get/save` (others refused, 256 KiB cap, asks nothing). Frame's `save` is the bridge's only write.

## Declarative content
- Layouts: ids `user:<name>` / `builtin:<name>` / `<plugin>/<layout>`; commands take the id, never the name. Same name listed once: user > plugin > shipped. Plugin layouts not deletable in the wizard. Install runs `import_templates::check_layout` on the mandatory sample (recognised, every wording mapped, no invalid row).
- Dictionaries (`provides.dictionaries`, `KindWords`, per language): asked **after** shipped keywords; shadowing a shipped word (`KindWords::shadowed`) or < 3 letters / 2 ideographs (`KindWords::too_short`) refused. Reach `build_preview` via `ImportContext::kind_words` (`ImportService::with_kind_dictionary`), not the mapping. Sample must be unreadable without it (`import_templates::check_dictionary`).
- Taxonomies: see `taxonomy-and-rebalance.md`.
- Themes: `UiState::theme` = `plugin:<plugin>/<theme>` (spelling owned by `lib/theme.ts`); base scheme via `data-theme`, stylesheet injected as the single `#plugin-theme` element. A preference naming a removed plugin is kept.
