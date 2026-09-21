# The AI assistant (`app/src-tauri/src/ai/`)

Host-only and one-way like the rest — the boundary itself is `.claude/rules/ui-boundary.md`, and
ADR-0037 says why the assistant lives in the host and not in `core`.

## Layering and the turn

- `ai/` holds the neutral types, the provider adapters, the SSE reader, the history layer and the
  loop, and none of them names Tauri — `tests/ai_layering.rs` checks that the way
  `grep -r tauri core/` checks the core. `commands/ai.rs` is the only Tauri file there.
- A provider id is the *stored* name of an adapter (`ai_chats.provider`, the keychain account),
  so `ai/catalog.rs` is the one place that maps it to one — `session::send` takes a
  `&dyn AiProvider` and never learns which it got, exactly as it never learns the model. Adding
  one is a file plus a row there — three are built in (OpenAI, Anthropic, Gemini). Where the
  adapters differ is the wire, not the loop: OpenAI reads the finished turn off
  `response.completed`, the others have no such envelope and assemble it from the deltas, and the
  `signature` on `Block::ToolCall`/`Block::Reasoning` exists only because a provider that signs a
  step refuses it handed back unsigned — Anthropic's thinking block, and Gemini's
  `thoughtSignature` on the function call itself.
- Gemini is the odd one in four specific ways, all of them in `ai/gemini.rs` and nowhere else: it
  takes a **subset of OpenAPI 3.0** rather than JSON Schema, so the catalogue's strict-mode
  schemas are translated on the way out (`gemini::schema` — an allowlist of fields, a union type
  becoming `type` plus `nullable`, `null` dropped out of an enum), and both `args` and a tool
  result must be objects, which is why anything else is wrapped rather than sent; a
  function call carries **no id**, so the adapter mints `name#index` and reads the name back out
  when the result is replayed (`functionResponse` names the function, not a call); a tool result
  is said by the *user* role, because there is no third one; and thinking is counted **beside**
  the answer (`thoughtsTokenCount` outside `candidatesTokenCount`), so the two are added to keep
  this app's rule that reasoning is part of output.
- Beside the two built-in providers there is **one the user configures** (`catalog::CUSTOM`,
  `AppSettings::ai_custom`): a label, a base URL, a `Wire` and a model id. It is a provider only
  once it has an address and a model, and its key is *optional* — a model served from the user's
  own machine authenticates nothing (`keys::for_call`). `Wire::OpenAiChat` is the default and the
  point of the feature: outside OpenAI itself, "OpenAI-compatible" always means
  `POST {base}/chat/completions`, which is `ai/compat.rs`. The other three wires are the existing
  adapters with their host replaced (`OpenAiProvider::at`, `AnthropicProvider::at`,
  `GeminiProvider::at`). No table of
  vendors ships in the binary. See ADR-0042.
- Tokens stream over a **Channel**, not `data:changed`: an ordered stream with one subscriber,
  where the channel is its own address. The turn runs on its own thread with its own `Store`,
  never holding `Mutex<Store>` across the network call.
- A failure is a code, not a sentence: `AiEvent` has no error variant, `session::send` returns
  `Err`, and the command maps it to `UiError` — `auth`, `rate_limit`, `provider` (it answered with
  an error of its own), `refused` and `truncated` are their own codes because the user's next
  action differs. A refusal or an output-limit stop is a `StopReason`, not an HTTP error:
  `StopReason::failure` turns it into one *after* the partial turn is saved, so what arrived stays
  in the chat and the error says why it ends there. No fallback model is tried.
- The provider keys live in the open profile's **vault** (`vault.json`, sealed under the profile's
  password, held unlocked in `AppState::vault` — `secrets.rs` is the only way in:
  `key_for_call`, `key_exists`, `key_save`, `key_delete`), a key is saved only behind a password,
  and **no command reads a key back** (ADR-0048); `ai::keys` only reads the pre-password
  device-wide entry for the `default` profile; `ai_key_status` answers connected/not, nothing more, and `ai_providers_list` folds the
  same answer into the picker so a provider without a key is never offered.
- What is true of the request rather than the assistant — date, screen (`ai_send` takes it,
  `useNav().screen` supplies it), lens, base currency — is `AiRequest::context`, rendered **after**
  the static system text: a moving value above the fixed one would cost the whole prompt on every
  request.
- A turn's cost is **counted, never priced**: `AiUsage` (`cached` inside `input`, `reasoning`
  inside `output`) reaches the panel as `AiEvent::Usage` carrying the turn's **running total**, so
  the receiver assigns it and never adds it up — providers report at different moments (OpenAI once
  per finished request, Anthropic as it streams) and a repeated or missed event must not shift the
  figure. One row per request in `ai_usage`, keyed to the model rather than the chat; the panel's
  figure resets with each new message, the lifetime total lives in Settings. Nothing is estimated
  from the text on screen, and no price table ships in the binary. See ADR-0041.

## The catalogue (`ai/tools/`)

- One entry per question rather than a call into `commands/`: parse arguments, take the scope, one
  `calc` call. A schema sits beside its body, one file per namespace, and `mod.rs` is the single
  index, in the order the prompt is paid for.
- Numbers cross as strings and identifiers are readable (a ticker, an account, a tree's name),
  never a UUID. The model does no date arithmetic: it names a shipped `period` and
  `ai/tools/args.rs` resolves it through the same axis as `period_ranges` (ADR-0018). The scope is
  snapshotted at `ai_send`, so moving the picker mid-turn does not change the question asked.
- Names carry the namespace of the command file they mirror (`portfolio_*`, `plans_*`, `alerts_*`,
  `watchlist_*`, `rebalance_*`, `report_*`, `app_*`), and each inherits its subject's scoping rule
  rather than a uniform one: `plans_*` reads the portfolio, `alerts_*` / `watchlist_*` /
  `market_quotes` / `securities_events` read the instrument, the rest read the lens. The system
  prompt says so — a model told "these accounts" would otherwise explain a watchlist as one of them.
- Anything the user named is matched case-insensitively, and a miss is a tool error naming what was
  not found, so the model corrects itself instead of the turn failing.
- A **write** reads the *portfolio*, never the lens — a row exists whether the picker is looking at
  it or not — and refuses an ambiguous match instead of picking one: `transaction_update` /
  `transaction_delete` name a row by date plus operation, instrument or account, and two rows
  answering to that come back as a tool error listing them. Editing the wrong operation is worse
  than editing none. A delete that would take data nobody mentioned says so: an instrument with
  operations against it is refused, and an account carrying them needs the call to ask for them
  explicitly. Import, corporate actions, attributes, exports and app settings stay outside the
  catalogue.
- Figures are rounded *where the tool answers*: `money` and `percent` to 2dp, a price to 6. Full
  ledger precision comes back quoted at 26 digits, and an answer is not truer for being long. The
  prompt's job beside that is wording — never a field name or a period code at the user
  (`twr_percent`, `ONE_YEAR`).
- How the app *works* is fetched, never carried: `app_reference(topic)` over
  `docs/ai-reference/*.md` and `app_user_guide(screen)` over `docs/user-guide/*.md`, both
  `include_str!`-ed into `ai/guide.rs`. Only the slugs — the schema's `enum` — stay in the prefix.
  Both are `Access::Free`, which is what `app_*` means: reads no portfolio data, so there is
  nothing to consent to. A corpus written *for the model*, not `.claude/rules` itself, because the
  model repeats what it reads and the user asking about dividends must not be told about a struct.
  Guide coverage is partial on purpose, so the test runs the other way — every file must name a
  live `ScreenId` (`guide::SCREEN_IDS`, pinned to `lib/nav.tsx`) — and "no guide" becomes "I do not
  know how that part works". See ADR-0038 and `.claude/rules/assistant-docs.md`.

## Consent

- Per tool, per chat, and **resolved by `request_id` out of `AppState::ai_consent`**. The frontend
  sends an id and one of `once | session | deny` and nothing else — never the tool, the arguments
  or any history, or what the user approved and what the host ran could be two different objects.
- `Access` is three-valued and the loop branches on it in exactly one place (`session::run_one`):
  `Access::Free` never asks and is never put in front of the user, `Access::Ask` is a read that a
  `session` grant or the chat's `AUTO` mode stands in for, and **`Access::Write` asks every single
  time** — no grant of any kind covers it. A write tool's body is the write command's body, side
  effects and all.
- `AiChat::tool_mode` (`ASK | AUTO`) is the chat's standing answer, read *per call* so "Allow all"
  answered mid-turn covers the next tool in the same reply. The footer toggle and the card's fourth
  button set that one field — one permission, two ways to reach it. `session` is the only grant
  that persists (`ai_grants`).
- The card has two halves from two authors, and they must not be confused. **What happens** is
  the host's: the tool's own `Params` (values, never a sentence) turned into text by
  `components/domain/aiToolLabels.ts`. **Why** is the model's: a `reason` argument the catalogue
  injects into every asking tool's schema in one place (`tools::definitions`), carried on
  `ToolRequested` and rendered quoted and dimmed, never on a button and never alone. A reason in
  the same call cannot drift from the call it explains, and no tool body reads it. It rides
  `ToolRunning` too, so a read that a `session` grant or `AUTO` mode waved through still says
  what it was for.
- The card is told *whether* the call writes (`ToolRequested::write`) and nothing more: a write
  card offers only "make this change" or "decline" and shows no standing permission, which the host
  would refuse to honour anyway. Its text is the frontend's, written from `Params` (values, not a
  sentence) in `components/domain/aiToolLabels.ts` — the model never writes the label on the button
  the user presses, and a card names the change, never the intention.
- A refusal is an *answer*: it goes back as a tool result and the turn keeps going. Every result is
  fenced in `<tool_data>` and the system prompt says what that fence means — the content came out
  of somebody else's CSV (`import.md`) and is data, never instruction.
- How the model got there is **shown, stored, and never replayed**: `Block::Reasoning` holds the
  provider's *summary* of its reasoning (no provider hands over the chain itself), streamed as
  `AiEvent::Reasoning` and folded away above the answer. Sending it back would hand the next step
  somebody else's notes — the same rule as `Block::WebSearch`, for a different reason. It is asked
  for only when `AppSettings::ai_reasoning` is on, and that default is **off**, unlike web search:
  the summary costs output tokens and an account not cleared for one has its whole request
  refused, so a feature nobody asked for cannot break a chat.
- Web search is the *provider's* tool: no schema and no body, declared by name
  (`AiRequest::web_search`), producing a `Block::WebSearch` that is shown but never replayed. It is
  `AppSettings::ai_web_search` rather than `UiState` because the host decides what leaves the
  machine.

## The panel

- A chat owns its **provider**, its model, its thinking effort and its tool mode — all four
  switchable in the footer and read *per step*, so one changed mid-turn applies to the next step of
  that same turn. Two chats open side by side may be answered by different providers;
  `AppSettings::ai_provider` only says where a *new* chat begins, and `ai_chat_create` starts it
  somewhere usable when that provider has no key. Switching provider carries the model
  with it (`Store::ai_chat_set_provider` writes both): a model id belongs to the catalogue it came
  from, and leaving one behind names a model the new provider has never heard of. Every provider
  this build can talk to is offered, connected or not — one without a key is listed and cannot be
  chosen, because a picker that hides the alternative reads as no choice at all.
- **No model is a setting.** A chat lands on the first model its provider lists *today*
  (`commands::ai::newest_model`); `catalog::Provider::default_model` is the fallback for when that
  list cannot be read, and the only model id a user ever types is their own server's, beside its
  address. An id compiled into a build outlives the model it names.
- The model list comes from the provider (`ai/models.rs`, cached per provider in
  `AppState::ai_models`, cleared for the custom one when its address changes), cut to **three**: the newest of each tier the catalogue already has
  (`gpt-…-sol / -terra / -luna` — or the older `gpt-… / -mini / -nano`, same slots — and
  `opus / sonnet / haiku`). The three are derived from what the provider
  answered rather than from a whitelist, so a model released after this build takes its tier's
  place by itself; a naming this file cannot read falls back to the first three of the ranked
  catalogue, and the chat's own model is always an option whatever the list says. Everything else
  a catalogue carries — dated snapshots, `-codex`, `-chat-latest`, transcription — answers a
  different question than "who answers this chat".
- A reading is shown while it happens: `AiEvent::ToolRunning` / `ToolFinished` / `Searching` drive
  the same component the persisted blocks do, so it does not change shape once saved. Consecutive
  readings are one rail (`components/domain/aiSteps.ts` builds the same `Step` from live events and
  from stored blocks), grouped **across turns**: a provider answering one tool call per turn would
  otherwise draw a dozen cards over one answer, and a long finished run folds itself away behind
  its count.
- Nothing in the panel carries a `data-tip`. A conversation explains itself, a hover bubble over
  one is noise, and the footer controls open the app's own menu (`useMenu`) rather than a native
  `<select>` — the platform list of a provider catalogue was as long as the catalogue.
- A chat titles itself from its first message (`session::title_from`, first line, cut at a word) —
  the user's own words, so it crosses IPC like a period's name. A chat already carrying history is
  never retitled: renaming by hand wins.
- Text renders as **markdown** (`components/ui/Markdown.tsx`), GitHub flavour, raw HTML
  deliberately not enabled (`rehype-raw` is absent): this is text a model wrote after reading an
  imported CSV.
- The panel's width is `UiState::ai_panel_width`, not `AppSettings` — only the frontend acts on it.
  Dragged with `lib/pointerDrag.ts`, followed in local state, written once on drag end; below 700px
  the panel is full-screen and the stylesheet ignores the number entirely.

## The dashboard's summary tile

- **Not** a chat: `ai_brief` gathers `brief::READINGS` in the host, sends them fenced with
  `tools: []` and asks once, so the model chooses nothing and one press costs one call, not a loop.
  Consent *is* the press: the tile lists what it reads before the first generation
  (`BRIEF_READINGS` in `lib/ai.ts`, pinned to the Rust list by a test) and never generates on its
  own. The text lives in `UiState::ai_briefs` keyed by widget id because it cost money, and is
  never invalidated — `lib/freshness` records the host's `data:changed` (a refetch is not a change,
  so the query cache cannot stand in) and the tile says it is out of date instead. `from`/`to`
  arrive resolved, like every other reporting command. See ADR-0039.
- The question, the rewriting interval and the language are the widget's own settings (`prompt`,
  `refresh` in `WidgetDef::fields`). The user's instruction is *appended* to the app's prompt, never
  swapped for it, and `READINGS` stays fixed in the host — a text box widens nothing. `refresh` is
  **off by default**, checked hourly rather than slept through, dead while the dashboard is not on
  screen, and never retried on a timer after a failure. The language crosses as the interface's
  locale *tag*, never a language name: a tile has no question to read the language off the way a
  chat does. See ADR-0040.
- The answer length (`cfg.max_tokens`, `brief::Options::max_tokens`) is shaped by the **prompt**,
  not by the ceiling: the model is told the budget in tokens and words, and `AiRequest::max_output`
  is that budget plus `REASONING_HEADROOM`, because every provider counts reasoning as output and a
  ceiling at the bare budget would truncate a thinking model before its first word. `None`
  everywhere else is `DEFAULT_MAX_OUTPUT`.
- The tile may name its own provider and model (`cfg.provider` / `cfg.model`, the `model` field,
  passed to `ai_brief`); absent, it follows `AppSettings::ai_provider` and `newest_model`. A model is
  stored only beside the provider it was picked from — switching provider clears it.
