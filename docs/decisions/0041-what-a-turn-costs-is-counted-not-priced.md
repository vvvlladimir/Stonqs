# 41: What a turn costs is counted, not priced

- Status: Accepted

## Context

The assistant spends the user's own money through their own key, and until now the app said
nothing about it. `AiTurn::usage` was parsed from the provider's response and thrown away.

Two questions need answering, and they are not the same question. *What did this answer cost?* is
asked while the answer is on screen, about the question just asked. *What has this key been
billed for?* is asked in Settings, about everything ever sent, and must survive a deleted chat.

A third question — *how much money is that?* — is deliberately not answered. A price table
compiled into the binary is wrong the first time the provider changes its pricing page, and a
wrong number about money is worse than no number.

Providers also report usage at different moments. OpenAI's Responses API counts once, in
`response.completed`; Anthropic reports input tokens at the start of a message and output tokens
as they stream. Whatever shape this takes has to hold both without the agentic loop knowing which
provider is underneath.

## Decision

`AiUsage` is a neutral struct in the core (`input`, `cached`, `output`, `reasoning`), where
`cached` is part of `input` and `reasoning` is part of `output`. An adapter fills what its
provider reports and 0 for the rest. It is the same struct the host streams and the same struct
SQLite stores — one shape, no conversion.

**It travels as an event, not a field.** `AiEvent::Usage` carries the running total *of the turn*
and is sent whenever anything is learned: the adapter sends what the provider reported, the
session adds it to the steps before it and sends the total again after each step. The event is
cumulative, so it is assigned and never accumulated by the receiver — a repeat or a drop leaves
the right number on screen.

**It is written down per request** (`ai_usage`, migration `0022`), because a request is what the
provider bills. `chat_id` is nullable and `ON DELETE SET NULL`: the dashboard brief has no chat
at all, and deleting a conversation does not un-spend what it spent. The totals in Settings are
grouped by model, which is the axis prices are quoted on.

**The panel shows the turn, Settings shows the lifetime.** A new message resets the panel's
figure; nothing in the UI adds one chat's turns together over the input box.

**Nothing is estimated.** While a single-step answer streams there is no figure to show, because
OpenAI has not counted yet — the panel shows a spinner beside the last known total rather than a
number derived from the text on screen. A guessed token count looks exactly like a real one.

## Alternatives

- **A field on `AiTurn` only.** Simplest, but a multi-step turn would report nothing until the
  whole loop finished, which is the case where the count matters most.
- **A delta event.** Matches how Anthropic streams, but any dropped or duplicated event corrupts
  the total silently; a running total cannot.
- **A row per turn.** Reads more naturally in a chat, but it is not what is billed, and the
  brief — one call, no turn, no chat — would have nowhere to go.
- **Cascade the usage rows with the chat.** Tidy, and wrong: it makes the lifetime total shrink
  when a user tidies their chat list.
- **Show money.** Rejected above: prices live on the provider's site, not in this binary.
- **Estimate tokens from the streamed text** (roughly characters ÷ 4, as some clients do). It
  would fill the quiet moment with a number that is not the provider's, in an app whose rule is
  that every figure comes from something that actually counted it.

## Consequences

- A second provider adds no column and no event: `openai.rs` reports what it has, a future
  `anthropic.rs` reports more often, and `session.rs` is unchanged.
- `usage.cached_tokens` is now visible, which is the direct check on the prompt-cache rule in
  `.claude/rules/ai-assistant.md` — a prefix that stopped matching shows up as a cached count
  stuck at zero.
- Spending is kept until the database is deleted. There is no "clear the counters" button; if one
  is ever wanted it is a deliberate action in Settings, not a side effect of deleting a chat.
- Token counts cross IPC as JSON numbers, not `Decimal` strings: they are counts, and nothing in
  the app does money arithmetic on them.
