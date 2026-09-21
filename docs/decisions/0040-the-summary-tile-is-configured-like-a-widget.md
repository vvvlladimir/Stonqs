# 40: The summary tile is configured like a widget, schedule included

- Status: Accepted

## Context

ADR-0039 built the dashboard brief as one reading with one fixed question, generated only on a
press, and rejected scheduled generation outright: money spent without the user asking.

Using it showed two things that argument got wrong.

**The question is not one question.** A tile that always answers "what happened and what drove it"
is one of several things a person wants standing on their dashboard — the drift from target in
words, what the dividends did this quarter, whether anything unusual happened. The widget already
configures its title, its data source and its period; the one thing it could not configure was the
thing it exists to do.

**"Never on a schedule" answered the wrong objection.** What is unacceptable is a tile that spends
money *without the user knowing it will*. A rewrite the user asked for — once a day, on a tile they
placed, in an interval they picked from a list that says what it costs — is not that. Refusing it
means a dashboard opened each morning shows yesterday's paragraph until someone presses a button,
which is worse for the user and no cheaper.

A third thing was simply a defect: the tile answered in whatever language the readings looked like,
and the prompt forbade every markdown construct, so a tile in a Russian interface would answer in
English prose with no emphasis.

## Decision

Three additions, all inside the widget's own settings dialog.

**`prompt`** — free text, appended to the app's own prompt, never replacing it. What the tile may
*state* — figures from the readings, no advice, no invention — is the app's rule and a text box on a
dashboard does not repeal it (`brief::with_instructions`, pinned by a test). Empty is the default
and keeps the original question.

**`refresh`** — off, daily, weekly, monthly. **Off is the default**, and the field says in plain
words that each rewrite is a paid request. The tile checks hourly rather than sleeping the whole
interval, so a laptop closed for a week notices on waking. A *failed* automatic rewrite never
retries on a timer — a rejected key would otherwise be asked about once an hour forever — and a
press is what gives it another chance. Nothing runs while the dashboard is not on screen, because
the widget is not mounted.

**`language`** — the command takes the interface's locale tag and the model answers in it. A tag,
not a language name: naming a language would be text crossing IPC, and a tile has no question to
read the language off the way a chat does.

The prompt now asks for light markdown — a short paragraph, bold on the figure that matters, a list
only when the answer is one — instead of forbidding all of it. The tile already rendered through
`components/ui/Markdown`; it was the prompt that made the renderer pointless.

## Alternatives

- **Keep one fixed question, add more tile types.** A widget per question is a catalogue entry per
  question, and the questions are the user's, not the catalogue's.
- **Replace the prompt instead of appending.** Simpler to explain and removes the one guarantee the
  tile makes about its own text.
- **Regenerate on `data:changed` rather than on a clock.** Rejected again, and this time for the
  reason ADR-0039 gave: a price tick is not a decision to spend, and quotes tick all day.
- **A global "refresh all briefs" setting.** Puts the cost of many tiles behind one switch nobody
  associates with the tile that spends it.

## Consequences

- ADR-0039's ban on scheduled generation no longer holds; everything else in it does — the fixed
  readings, `tools: []`, the press as consent for the first generation, storage in
  `UiState::ai_briefs`, and staleness reported rather than acted on.
- A tile can now cost money on a timer. The bound is explicit: at most one call per interval per
  tile, only while the dashboard is open, and never after a failure until a press.
- A user's instruction reaches the model. It is their own words about their own tile — the same
  standing as a chat message — but it is *not* a way to widen what the tile reads: `READINGS` is
  fixed in the host and the request still carries no tools.
- `Field` gains `prompt` and `refresh`, so any future widget that writes prose gets both by
  naming them.
