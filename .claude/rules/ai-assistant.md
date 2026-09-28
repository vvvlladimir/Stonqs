---
paths:
  - "app/src-tauri/src/ai/**"
  - "app/src-tauri/src/commands/ai/**"
  - "app/src-tauri/src/plugins/tool.rs"
  - "app/src-tauri/tests/ai_*"
  - "app/src/lib/ai.ts"
  - "app/src/components/domain/ai*"
  - "app/src/components/domain/aiChat/**"
  - "app/src/screens/dashboard/widgets/**"
---

# AI assistant (`app/src-tauri/src/ai/`, ADR-0037)

## Layering and providers
- `ai/` never names Tauri (`tests/ai_layering.rs`); `commands/ai/` is the only Tauri code.
- `ai/catalog.rs` maps a stored provider id to an adapter; `session::send` takes `&dyn AiProvider`. Built-in: OpenAI (turn from `response.completed`), Anthropic, Gemini (assembled from deltas). `signature` on `Block::ToolCall`/`Block::Reasoning` exists because Anthropic thinking and Gemini `thoughtSignature` must be handed back signed.
- Gemini quirks, all in `ai/gemini/`: OpenAPI 3.0 subset (`gemini::schema` allowlist, union → `type`+`nullable`, `null` out of enums); `args`/results must be objects (wrap); no call id → mint `name#index`; tool results use the user role; `thoughtsTokenCount` added to output. 429: daily/`limit: 0` quota = provider error; 429 with `googleSearch` retried once without it, then no search for that model this run.
- Custom provider (`catalog::CUSTOM`, `AppSettings::ai_custom`, ADR-0042): label, base URL, `Wire`, model; key optional (`keys::for_call`). `Wire::OpenAiChat` (default) = `ai/compat/` `POST {base}/chat/completions`; others reuse adapters via `::at`. No vendor table ships.
- Tokens stream over a Tauri **Channel**. The turn runs on its own thread with its own `Store`, never holding `Mutex<Store>` across network.
- Failure is a code: `AiEvent` has no error variant; `session::send` → `Err` → `UiError` (`auth`, `rate_limit`, `provider`, `refused`, `truncated`). `StopReason::failure` converts refusal/length stop *after* the partial turn is saved. No fallback model.
- Keys live in the profile vault; `secrets.rs` is the only access (`key_for_call`, `key_exists`, `key_save`, `key_delete`); saved only behind a password; **no command reads a key back** (ADR-0048). `ai::keys` only reads the legacy device entry for `default`. `ai_key_status` = connected or not.
- Request facts (date, screen via `useNav().screen`, lens, base currency) go in `AiRequest::context`, rendered **after** the static system text (cache).
- Usage counted, never priced (ADR-0041): `AiUsage` (`cached` ⊂ input, `reasoning` ⊂ output); `AiEvent::Usage` carries the running total (receiver assigns, never adds). One `ai_usage` row per request. No price table.

## Tool catalogue (`ai/tools/`)
- One entry per question: parse args, take scope, one `calc` call. Schema beside body, file per namespace; `mod.rs` is the index in prompt order.
- Numbers as strings; identifiers readable (ticker, account/tree name), never UUIDs. Model names a shipped `period`; `ai/tools/args.rs` resolves it (ADR-0018). Scope snapshotted at `ai_send`.
- Namespaces mirror command files (`portfolio_*`, `plans_*`, `alerts_*`, `watchlist_*`, `rebalance_*`, `report_*`, `app_*`), each with its subject's scoping (see `scope.md`); the system prompt says so.
- User-named things matched case-insensitively; a miss is a tool error naming it.
- Writes read the *portfolio*; ambiguous matches (`transaction_update`/`_delete` by date + op/instrument/account) are refused listing candidates. Deleting an instrument with operations is refused; an account with operations needs explicit opt-in. Import, corporate actions, attributes, exports and settings are outside the catalogue.
- Round at the tool: money/percent 2dp, price 6dp. Prompt: never show field names or period codes to the user.
- `app_reference(topic)` / `app_user_guide(screen)` over `include_str!`-ed docs in `ai/guide.rs`; only slugs sit in the schema; `Access::Free` (see `assistant-docs.md`, ADR-0038).
- Plugin tools (ADR-0085): not `CATALOGUE` rows; `Plugins::tools()` per message into `Session::plugin_tools`; `session::Entry` branches in `run_one`. Always `Access::Ask`. WASM body gets `plugins::reads::project` in bridge field names (`the_projection_is_the_bridges` guards drift, ADR-0088); host adds `reason` and `period`. Schema checked at install against the strict subset (`required` present, even empty); duplicate model name across plugins refused. Model sees `plugin_<plugin>_<tool>`, description "From the … plugin, not the app", answer `{ plugin, tool, answer }`; card label `PLUGIN_TOOL_LABEL`.

## Consent
- Per tool, per chat, resolved by `request_id` from `AppState::ai_consent`; frontend sends only id + `once|session|deny`.
- `Access` branches in one place (`session::run_one`): `Free` never asks; `Ask` covered by a `session` grant or `AUTO`; **`Write` asks every time**.
- `AiChat::tool_mode` (`ASK|AUTO`) read per call; footer toggle and card's fourth button set the same field. Only `session` grants persist (`ai_grants`).
- Card: *what* = host's `Params` → text in `components/domain/aiToolLabels.ts`; *why* = model's `reason` arg injected by `tools::definitions`, carried on `ToolRequested`/`ToolRunning`, shown quoted/dimmed, never on a button, read by no tool body. `ToolRequested::write` → card offers only apply/decline.
- Refusal is a tool result; the turn continues. Results fenced in `<tool_data>` = data, never instructions.
- `Block::Reasoning` (provider's summary) shown, stored, **never replayed**; only when `AppSettings::ai_reasoning` (default **off**). Web search: provider tool by name (`AiRequest::web_search`), `Block::WebSearch` never replayed; `AppSettings::ai_web_search` (host decides what leaves the machine).

## Panel
- A chat owns provider, model, effort, tool mode; read per step. `AppSettings::ai_provider` = where a new chat starts; `ai_chat_create` falls back to a keyed provider. `Store::ai_chat_set_provider` writes provider + model together. Unkeyed providers listed but not selectable.
- New chat defaults (ADR-0069): remembered `ai_provider`, `ai_models` (per provider), `ai_effort`; else smallest tier (`commands::ai::providers::default_model` → `models::smallest`). `models::remembered`: same id else newest of its tier. `catalog::Provider::default_model` only when no list. Tool mode not carried; `settings_save` never overwrites remembered picks.
- Model list (`ai/models/`, cached in `AppState::ai_models`; custom cleared on address change): newest per tier (`-sol/-terra/-luna` or `/-mini/-nano`; `opus/sonnet/haiku`), derived, not whitelisted; unknown naming → first three ranked; chat's own model always offered. `AppSettings::ai_extra_models` appended after the cache in `models_for`, unvalidated, count as on offer.
- Live `ToolRunning`/`ToolFinished`/`Searching` drive the same component as stored blocks; `components/domain/aiSteps.ts` builds `Step`s for both, grouped across turns; long runs fold.
- Standing disclaimer `.ai-note` under the composer and `.brief__note` under the brief — not dismissible (ADR-0078).
- No `data-tip` in the panel; footer controls use `useMenu`, not `<select>`.
- Title from first message (`session::title_from`); never retitle a chat with history.
- Markdown via `components/ui/Markdown.tsx`, GFM, no raw HTML (`rehype-raw` absent).
- Width `UiState::ai_panel_width`, dragged via `lib/pointerDrag.ts`, saved on drag end; < 700px full-screen.

## Dashboard brief (ADR-0039/0040)
- Not a chat: `ai_brief` gathers `brief::READINGS`, sends fenced with `tools: []`, one call. The press is consent; `BRIEF_READINGS` in `lib/ai.ts` pinned to Rust by a test; never auto-generates on first view.
- Text stored in `UiState::ai_briefs` by widget id, never invalidated; `lib/freshness` records `data:changed` and the tile says it's stale. `from`/`to` arrive resolved.
- Widget settings `prompt` (appended, never replaces), `refresh` (off by default, hourly check, only while visible, no retry after failure), language as locale tag.
- Length shaped by prompt; `AiRequest::max_output` = budget + `REASONING_HEADROOM` (`None` elsewhere = `DEFAULT_MAX_OUTPUT`).
- Optional `cfg.provider`/`cfg.model`; model stored only beside its provider, cleared on provider change.
