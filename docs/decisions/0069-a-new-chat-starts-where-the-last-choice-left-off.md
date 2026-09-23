# 69: A new chat starts where the last choice left off

- Status: Accepted

## Context

ADR-0042 removed the model from the settings: a chat landed on the first model its provider listed
that day, which is the newest flagship. Two things followed that users did not want. Every new chat
started on the most expensive tier, whatever the question was, and a model, provider or thinking
effort switched in one chat's footer was forgotten by the next one, so the same three clicks were
repeated per conversation.

ADR-0042's reason still holds: an id written down once outlives the model it names, and replaying
a retired id makes the first send fail.

## Decision

- With nothing chosen yet, a chat starts on the provider's **smallest tier** as its catalogue lists
  it today (`models::smallest`: `-luna` / `-nano`, Haiku, Flash-Lite). The compiled-in fallback
  used when the catalogue cannot be read is the small tier too. The custom provider starts on the
  model the user typed, as before.
- Switching the provider, model or effort in a chat's footer is **remembered** in `AppSettings`
  (`ai_provider`, `ai_models` per provider, `ai_effort`) and is where the next chat starts. Switching
  a chat to another provider lands on the model last picked there.
- A remembered model is read against today's list (`models::remembered`): the id itself while it
  is offered, else the newest model of the **same tier**, else the smallest tier. Only when no
  list can be read is the stored id used as it is.
- The dashboard brief, with no model of its own, follows the same rule.
- The **tool mode** is not carried. ADR-0037 stands: a chat opened permissively must not make the
  next one permissive.
- `settings_save` keeps the stored `ai_models` / `ai_effort`; only the chat commands write them.

## Alternatives

- **A model picker in Settings.** A second place to choose the same thing, and it would still
  need the "retired id" rule; remembering the footer's choice needs no new control.
- **Carrying the tool mode too.** Rejected for the consent reason above.
- **Remembering only the last model, not one per provider.** A model id belongs to one catalogue,
  so switching provider would drop the choice made on the other one.

## Consequences

- `settings.json` gains `ai_models` and `ai_effort`; an older file reads them as empty / `MEDIUM`.
- The footer's provider switch now also changes which provider Settings shows for new chats.
- A new chat costs the small tier's price by default; the user opts up once and it sticks.
