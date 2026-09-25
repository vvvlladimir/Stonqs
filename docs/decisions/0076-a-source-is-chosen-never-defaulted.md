# 76: A data source is chosen, never defaulted

- Status: Accepted

## Context

Until now a fresh install fetched quotes from Yahoo Finance and rates from the ECB without
being asked. `sources::DEFAULT_QUOTES` named Yahoo in three places — the import's new
instruments, the venue picker, the assistant's tool — and `SourceInfo::on_by_default` was `true`
for every source the build could reach.

Two things are wrong with that. The first is whose decision it is: prices come from services
other people run, under their own terms, and a build of ours that starts asking one on behalf of
somebody who never named it makes that agreement for them. The second is ours: naming one
provider as *the* default reads as an arrangement with them, and there is none.

The venue picker still has to offer the sources in a sensible order, and the order that is
useful puts the broadest one first. "First in the list" and "chosen for you" had been the same
constant, and they are not the same thing.

## Decision

No source is asked until the owner has said which may be asked.

- `DEFAULT_QUOTES` is gone. `sources::default_quotes(&Setup) -> Option<&'static str>` answers
  which source a new instrument is stamped with: the first quote source that is **on**, and
  `None` while none is. An instrument created then carries no source and is priced by hand.
- `quote_ids` returns the on sources in catalogue order, which is where Yahoo is first. Being
  first is a position in a list, not a recommendation, and nothing in the interface calls a
  source recommended.
- No quote source is `on_by_default`. The rate and price-index sources of public institutions,
  which publish for reuse, keep theirs.
- `AppSettings::sources_configured` is the owner's answer, and **turning a source on is that
  answer**: `market_source_switch(.., on = true)` sets it. Naming a service that may be asked is
  the whole of what is being asked for, and a switch that fetches nothing until a second press
  elsewhere confirms it reads as broken. Turning one *off* answers nothing — it narrows an answer
  already given, and the last source off is still an answered state, priced by hand. While it is
  `false`,
  `AppState::market_setup` writes every switch to `false` — over the catalogue's defaults and
  over the stored map alike — so every reader of a `Setup` is gated by one line, and
  `jobs::start` refuses outright rather than reporting a failure per instrument.
- `SourceInfo::site` is the provider's own address, shown beside its switch. Turning a source on
  is the owner's business with whoever answers it, and the app says where the requests go.
- `AppSettings::version` (`SETTINGS_VERSION`) carries the reading of a stored file. A file
  written before this one meant the old defaults by its silence, so `settings::migrate` writes
  that state down and marks the owner as having chosen: somebody whose quotes arrive today must
  not find them stopped by an update.

The question is asked once, by the sources dialog a new profile opens **before** the tour is even
offered (ADR-0077) and by the banner on the market-data settings. Turning nothing on is an answer too, and
it is a stated one rather than a silent one:

- The dialog seals a set only when it can price a portfolio — one source of quotes and one of
  exchange rates. Either alone leaves every holding in a foreign currency unvaluable, which is
  not a smaller number but a missing one, so `Use these sources` is refused and names what is
  missing. `Select all and continue` is the one press that answers the whole question.
- The settings panel has no such press: its rows *are* the picker, so it states what is still
  missing and the `Turn on` beside a source is what removes the statement.
- Leaving without choosing stays possible (`Decide later`), and the refresh chip then carries the
  error: it reads `No data source`, says which half is missing, and opens this dialog instead of
  starting a refresh that would fetch nothing.

## Alternatives

**Leave Yahoo as the default and add a switch.** What we had. The switch does not help somebody
who never opens that screen, which is exactly the person on whose behalf the request is made.

**Ship no source in the binary at all.** Then every install begins with the user describing a
feed by hand (ADR-0054), which is a research task, not a setup step.

**Flip `on_by_default` and nothing else.** A stored `market_sources` map would then re-enable a
source by *default* rather than by choice, and the "never asked yet" state would be
indistinguishable from "asked and answered with the defaults". The flag is what tells them apart.

**Pre-tick the sources in the dialog.** A pre-ticked box that sends requests is a default wearing
a checkbox. The dialog offers `Select all and continue` instead — one press, clearly the user's.

## Consequences

- A new profile fetches nothing until the dialog is answered. Prices are typed in by hand until
  then, which is a supported state and now a visible one: the chip that would otherwise report a
  refresh reports the unanswered question instead, and a set answered with too little to work
  reads the same way as one never answered, because the app fetches nothing either way.
- An instrument created while no source is on carries none, and no later refresh would notice it.
  The dialog offers those instruments the first source that arrives, as a checkbox, once.
- `ImportOptions::new_security_source` defaults to `None`; the caller names the source. The
  import wizard already offered the first active provider, so nothing changes there.
- The assistant's `ToolContext` carries `quotes_source`, read from the switches on the command
  thread — a turn runs on its own thread and holds no lock on the settings.
- The CLI names `YahooProvider::ID` outright. It is a harness that registers the provider two
  lines later; an explicit name there is not a shipped default.
