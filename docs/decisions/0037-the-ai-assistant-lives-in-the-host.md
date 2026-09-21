# 37: The AI assistant lives in the host

- Status: Accepted; key storage superseded by ADR-0046

## Context

The assistant is a chat panel that answers questions about the portfolio. Answering means three
things this repository keeps out of `sq-core` on purpose: a network call, a user's secret, and a
decision the user has to consent to. It also means chat history, which is user data and therefore
does belong in the database.

`market/` and `fx/` live in `core` because `core` *uses* them — a valuation cannot be computed
without quotes. Nothing in `core` will ever call a model.

## Decision

The assistant is `app/src-tauri/src/ai/`, a module of `sq-app`. Inside it:

- `mod.rs` fixes the neutral vocabulary — `Role`, `Block`, `StopReason`, `Effort`, `AiRequest`,
  `AiTurn`, `AiEvent`, the `AiProvider` trait and `AiError`.
- `openai.rs` is the only file that knows the Responses API; `sse.rs` is wire framing alone.
- `store.rs` is the history layer: neutral turns in, neutral turns out, over `sq_core::Store`.
- `tools.rs` is the catalogue, `consent.rs` the gate, `session.rs` the agentic loop, `keys.rs`
  the keychain, `fake.rs` the offline double.
- `commands/ai.rs` is the only file here that knows Tauri, and `tests/ai_layering.rs` enforces
  that: everything above it is checked for the word, comments stripped.

Consequences of that split, each of which is the real decision:

- **The provider is OpenAI in v1, but the stored shape is neutral.** `ai_messages.content` holds
  JSON of `Block`, never a provider's own payload. This is the one place the repository builds an
  abstraction before seeing a second example, because the cost of guessing wrong is not a refactor
  but incompatibility with chat histories already on users' disks. The shape follows the de-facto
  OpenAI-style form that LiteLLM and the Vercel AI SDK both normalise to.
- **The key never crosses IPC.** It is written to the OS keychain (`keyring`, service
  `app.stonqs.desktop.ai`, account = provider id) by `ai_key_save` and read only by the host, on the
  thread that is about to make the call. There is no command that returns a key — that absence is
  the invariant, not an omission. `settings.json` carries `ai_enabled`, `ai_provider`, `ai_model`
  and nothing secret.
- **`keyring`'s platform features are part of the decision, not packaging detail.** With none of
  them enabled the crate silently falls back to an in-memory mock store where every `Entry` is
  fresh, so saving a key appears to work and keeps nothing. The dependency names
  `apple-native`, `windows-native` and `sync-secret-service` explicitly.
- **A failure crosses as a code, like every other failure.** `AiEvent` has no error variant;
  `session::send` returns `Err`, and `commands/ai.rs` maps it to `UiError` before it reaches the
  channel (`AiStreamEvent::Error`). 401 and 429 get codes of their own (`auth`, `rate_limit`)
  because the user's next action differs. The English strings in `ai/` are developer detail that
  a frontend sentence quotes, never the sentence itself — ADR-0023.
- **Streaming is a Channel, not an event.** `data:changed` is a broadcast several places listen
  to; tokens are an ordered stream with one subscriber, and the channel is its own address.
- **The turn runs on its own thread with its own `Store`.** `Mutex<Store>` is never held across a
  network call, the same rule and the same shape as `jobs.rs`.
- **Consent is resolved by id out of host state**, never from what the frontend echoes back, and
  the card's wording is the frontend's. Both halves matter: the first keeps "approved" and "ran"
  the same object, the second keeps the model out of the text on the button. The loop's step
  budget (8) is the other spending limit beside `effort`.
- **A curated shortlist is not what makes the assistant trustworthy — the gate is.** So the
  catalogue covers the questions rather than ten hand-picked ones, and the narrow thing is
  consent: per tool, per chat, asked the first time and remembered only as `session`.
- **A tool result is fenced and declared to be data.** Instrument names and operation wordings
  arrive from imported CSVs, so the fence plus the system prompt line is the standing answer to
  prompt injection; a refusal, a missing tool and a failed tool are all answers handed back to
  the model rather than errors raised at the user.

## Alternatives

- **A `vs-ai` workspace crate.** It could not hold the part that matters — a tool body needs the
  portfolio and the active scope, which live in `sq-app`, so the crate would cycle. What is left
  is transport, and a crate boundary around transport guards nothing. Keeping those files
  Tauri-free, and testing that they are, leaves the extraction a `git mv` if a second consumer
  (`sqcli ask …`) ever appears; `tools.rs` takes `(Store, ScopeSelection, args)` for the same
  reason, so an MCP server later is a different caller of the same bodies.
- **`tauri-plugin-stronghold` for the key.** A secrets plugin is for secrets the *frontend* reads;
  ours is read only by Rust. Stronghold would add a second cryptosystem and a second password
  where the OS already offers exactly what is needed.
- **Assembling the turn from stream deltas.** `response.completed` carries every finished item,
  so deltas are used for the live text and nothing else. There is no partial JSON to reassemble
  and no ordering assumption about parallel tool calls to get wrong.
- **An async provider (`rmcp`, an MCP server) now.** MCP is a transport between processes; the
  model and the data are in one process. It would also pull `tokio` into a deliberately
  synchronous workspace. When it is wanted it goes in a binary of its own.

## Consequences

- Adding a second provider is one file plus a `match`: the loop, the storage and the UI do not
  move. What a chat is made of does not change with who answers it, so a provider may even change
  mid-chat — `ai_chats.provider`/`model` record what a chat was *started* with.
- Chat history is not encrypted, exactly as the rest of the database is not. The settings screen
  therefore owes the user a way to delete a chat and to delete all of them.
- The catalogue is wide and the gate is narrow, and that is the trade deliberately made: every
  question the assistant can answer is a tool, and trust is handled once, per tool per chat, by
  the user rather than by a curated shortlist nobody can justify the edges of. Adding a tool is
  an entry in `CATALOGUE` and a label in `aiToolLabels.ts` — a tool with no label still shows its
  name, because a blank card is worse than an untranslated one.
- Consent has two grades above "once", and both live on the chat: a `session` grant per tool
  (`ai_grants`) and `AUTO` for all reads (`ai_chats.tool_mode`). The mode toggle and the card's
  "Allow all" are the same field, so the two controls cannot come to mean different things, and
  the mode is read per call rather than per turn — an answer given mid-reply has to apply to the
  next tool in that reply. Neither grade is a global setting: permissiveness does not outlive the
  conversation it was granted in.
- The assistant gives opinions. An earlier draft forbade recommendations in the system prompt;
  that produced answers hedged into uselessness for a tool whose whole point is reading one
  person's own portfolio. What is kept is the part that is about honesty rather than liability:
  every figure comes from a tool, and judgement is labelled as judgement.
- Write tools are in the catalogue, and the rule that keeps them safe is one `match` in
  `session::run_one` rather than a note: `Access::Write` always asks, and answering its card with
  "allow all" still only means this once. A write tool's body is the write command's body —
  the same validation, the same consequences (`security_set_data_source` drops the quotes stored
  under the old symbol, exactly as switching a listing does) — because a second, lighter write
  path is where the invariants in `money-and-fx.md` would quietly diverge.
- Rounding belongs to the tool, not the prompt. A model handed 26 significant digits quotes 26
  significant digits back; asking it nicely not to is a rule that fails silently, while rounding
  at the boundary cannot. What the prompt owns is wording — the user has never seen the JSON and
  should not be able to tell it exists.
- Web search is the provider's own tool: declared by name, executed on their side, recorded here
  as `Block::WebSearch` for display and never replayed into a later request. It is a setting
  rather than a default because it is the one thing in this feature that sends the user's own
  question somewhere other than the model.
