# 91: The sources question is a step of onboarding

- Status: Accepted

## Context

ADR-0076 and ADR-0077 put the sources question in a dialog the shell opened over a new profile,
before the tour was offered. So the first thing somebody saw after `Create portfolio` was the
app drawn behind a modal asking something the gate could have asked: a second screen of setup,
dressed as an interruption.

## Decision

The onboarding gate asks both, one after the other, on the same card.

- Step one is the portfolio form; its button is `Continue` and it writes nothing. The demo
  button (`Try with a demo portfolio`) leads to the same second step.
- Step two is the sources picker (`useSourcesSetup`, the same body and buttons the dialog has),
  with `Back` to the form. Leaving it — `Use these sources`, `Select all and continue` or
  `Decide later` — writes the answer first and **then** the portfolio or the demo, so the shell
  never appears before the question was put. A switch pressed in the picker is still written at
  once (ADR-0076); only the seal and the portfolio wait.
- The tour no longer opens the sources dialog. Every new profile passes the gate, the demo
  profile of Settings → *Getting started* included, so the offer is shown as soon as the shell is.
- `SourcesSetup` stays as the dialog the refresh chip opens while the set cannot price anything.

## Alternatives

**Create the portfolio on step one and keep the gate up until the question is answered.** The
gate is chosen by `account_count === 0`; keeping it after the write needs a second condition held
outside the query, and a reload in between would drop straight into the shell with the question
unasked.

**Keep the dialog, only open it on the gate.** Still a modal over a card that has room for it.

## Consequences

- A failure of `setup_portfolio` is shown on step two, beside the buttons; `Back` returns to the
  form with what was typed.
- `Decide later` leaves the profile exactly as before: nothing sealed, the chip says so.
- ADR-0076's "the sources dialog a new profile opens" and ADR-0077's ordering rule are replaced
  by this one; the rest of both stands.
