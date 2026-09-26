# 77: The guided tour is a declaration over the real screens

- Status: Accepted

## Context

Somebody opening the app for the first time meets an empty portfolio and seventeen screens, and
nothing says which of them answers what. The demo portfolio has existed for a while but only as
an offer on the onboarding gate — it fills the screens with figures and explains none of them.

A tour can be built three ways: a separate screen that describes the app, a recorded walkthrough,
or an overlay on the app itself. The first two are a second copy of the interface, and a copy
goes stale at the first change without anybody noticing.

## Decision

The tour is a list of steps over the screens the app already has.

- `lib/tour/steps.ts` is the whole content: a step is a screen, an optional `data-tour` anchor,
  a title and a body. `lib/tour/index.tsx` holds the state and draws nothing;
  `components/domain/tour/` draws and holds no state — the split `lib/updates.tsx` already uses.
- **A step never names an instrument, an amount or a figure.** The same steps run over the demo
  portfolio a new profile is filled with and over somebody's real one, and a sentence about a
  number would be wrong in one of them. A step says what a control is for.
- A named anchor that never appears is **skipped**, not waited for. A widget can be deleted from
  the board and a dock control can be absent on a narrow window; neither may stall the tour.
- A step with no anchor is about its screen as a whole and its card is centred. That is why the
  tour needs a handful of `data-tour` attributes rather than one per screen.
- The tour **writes nothing**: no portfolio data, and no setting but the one recording that the
  offer was answered (`UiState::tour`). A stored layout with no `tour` belongs to somebody
  already using the app, so it reads as answered — the offer is for new profiles.
- It is offered once per profile and started again by the `tour` command, which puts it in the
  palette, the shortcut list and the macOS **Help** menu by itself, and by Settings →
  *Getting started*, which also opens a demo profile of its own so somebody can poke at data
  that is nobody's money.
- A new profile is shown the sources dialog (ADR-0076) **before** the offer, not after the last
  stop and not behind a `Show me around`: it is the one question nobody else can answer, a tour
  walked with nothing fetched points at screens whose figures are all absent, and an offer drawn
  over that dialog asks two things at once. The offer comes up once the dialog is closed, however
  it was closed — declining to choose is an answer to it, and the stops do not depend on what was
  chosen. Neither is shown again by itself: answering the offer is what records the profile as
  asked.

## Alternatives

**A tour screen of its own.** A second description of the interface, drifting from it from the
first commit, and it teaches the tour rather than the app.

**Video or screenshots.** Stale at the first restyle, untranslatable without re-recording, and
useless to somebody who wants to try the control while it is being explained.

**Point at everything.** Every screen would need anchors on its own controls, which is a
`data-tour` attribute on a few hundred elements and a step list nobody reads to the end. Twelve
stops, most of them about a screen rather than a button.

**Drive the tour with real clicks.** A tour that presses buttons is a tour that writes data.

## Consequences

- Adding a stop is a row in `steps.ts`, plus a `data-tour` attribute when it points at something.
- Renaming or removing an anchored control silently loses a stop — it is skipped, not broken.
  That is the intended failure: a tour that stops the app is worse than a tour missing a step.
- The tour holds the keyboard through `useLayer`, so Escape leaves it and commands are off
  beneath it, like any other overlay.
- Both corpora the assistant reads stay the authority on what a figure *means*; the tour says
  what a screen is *for*, in one paragraph, and never repeats a number's definition.
