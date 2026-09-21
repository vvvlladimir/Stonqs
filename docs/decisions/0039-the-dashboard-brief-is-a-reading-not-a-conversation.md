# 39: The dashboard brief is a reading, not a conversation

- Status: Accepted; the ban on scheduled generation is superseded by ADR-0040

## Context

The dashboard should be able to say, in prose, what the period did and what drove it. The obvious
implementation is the assistant that already exists: a tile that asks the chat a standing question.

Three things make that wrong in a tile rather than in a panel.

**Cost has no user watching it.** A chat turn costs what the user asked for, right after they asked.
A tile renders whenever the dashboard is opened, and an agentic loop costs up to eight model calls
per generation. Anything automatic here is money spent while the user is looking at something else.

**Consent has nothing to attach to.** ADR-0037 resolves consent per tool, per chat, by request id,
because the *model* chooses which tool to call and the user must approve that exact choice. A tile
has no chat and asks no question of its own; there is nothing for a per-call card to be about, and a
card per render would be an interrogation.

**A summary needs no agency.** Which readings answer "what happened this period" is known when the
widget is written. Letting a model discover them buys nothing and costs a loop.

## Decision

The brief is **one model call over a fixed set of readings gathered by the host first**.

`ai/brief.rs` names `READINGS` — currently the portfolio overview, the period's performance and the
largest positions — runs those tool bodies itself, fences the results in `<tool_data>` exactly as a
tool result is fenced, and sends them as the first message with **`tools: []`**. The model chooses
nothing, so nothing it says can widen what was read. The reading bodies are the catalogue's own
(`tools::portfolio_overview`, `performance_over`, `positions_list`), not a second path to the same
figures.

Consent is the button. The tile lists what it will read before the first generation and generates
only on a press, so every generation is an act of the user's — stronger than a remembered grant,
and it needs no storage. `BRIEF_READINGS` in `lib/ai.ts` names the same list for that card and is
pinned to the Rust one by a test.

`ai_brief` takes a **resolved window** (`from`/`to`) like every other reporting command, and an
optional `source` like every other widget: the tile already knows the dates it was drawn for, and
resolving its period a second time in the host could disagree with them.

The result is stored in `UiState::ai_briefs`, keyed by widget id, with the time it was written and
the window it describes. It survives a restart because it cost money. It is **never invalidated**:
when the figures under it move, `lib/freshness` — an in-memory record of the host's own
`data:changed`, which a query cache cannot stand in for because a refetch is not a change — makes
the tile say it is out of date and wait. A tile that regenerated itself on a price tick would be
the one thing this design exists to prevent.

## Alternatives

- **A hidden chat per tile.** Reuses the loop, and inherits everything about it that a tile does not
  want: history that grows, a consent gate with nothing to ask, and a cost per render.
- **A tool the model calls to write the summary.** Backwards — the tile's question is fixed, so the
  only thing a catalogue would add is the chance of a different set of readings each time.
- **Generating on a schedule or on data change.** Rejected outright: money spent without a press.
  Half of this is revisited in ADR-0040 — an interval the user picks in the tile's own settings is
  not "without a press"; generating on data change still is.
- **Keeping the text in React Query only.** Free, and loses a paid-for paragraph on every restart.

## Consequences

- The brief's cost is one call with `effort: High`, bounded and predictable, and the user can see
  exactly what a press will read before pressing.
- Adding a reading to `READINGS` changes what the user agreed to. It is a wire format: change the
  Rust list, the TS list and the card together, which the pinning test enforces.
- A brief can be about a period the tile no longer shows (the user changed the widget's period
  afterwards). That is stated rather than hidden, and a press fixes it.
- Staleness is per session. After a restart, an old brief reads as old by its timestamp alone —
  the startup refresh announces whatever changed while the app was closed, which restores the mark.
- The brief writes prose from figures the host chose, so ADR-0038's rule still holds: everything
  it may state is in front of it, and there is no tool to reach for anything else.
