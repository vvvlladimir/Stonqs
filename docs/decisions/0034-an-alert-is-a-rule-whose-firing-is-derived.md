# 34: An alert is a trigger level with a crossing log; events ride on the quote request

- Status: Accepted

## Context

Some trackers let a user set a limit price on an instrument ("tell me when it rises to
17"), attach a date to act on ("review on 1 July"), and see dated events — its own notes, dividends
and splits — on the instrument's chart and as dashboard widgets. There it is done with an attribute
of type *Limit Price* holding `> 17`, one limit per attribute, and it shows only what is beyond its
limit today. It tells nobody.

We want the same answers, plus an OS notification, on desktop and mobile from one code base.

- A level is watched for a reason: a stop-loss cares about the fall, a take-profit about the rise,
  a review level about both. The direction is part of the rule, not of the price.
- A notification has state: it must be shown once per crossing, and again for the next one.
- The host knows no language (ADR-0023), so it cannot write the notification text.
- The startup refresh can finish before any window listens (ADR-0014), so a pushed event is lost.
- Our only quote provider already has dividends and splits in the response we request.

## Decision

**A price alert is a level with a direction.** `security_alerts` holds `PRICE` (a level, the currency
it is written in, and `UP`, `DOWN` or `BOTH`) or `DATE_REACHED` (a day). A close at or above the level is *above*,
anything else *below*; a crossing is a change of side between two closes. The bookmark follows
every change of side, but only a crossing in the rule's direction is logged: a rise-above rule
stays quiet while the price falls back and fires again on the next rise. The first close read —
the one on or before the day the rule was created — only sets the side: the trigger starts from
where the price is. A quote in another currency is converted at the rate of its own day.

**Crossings are logged, not re-derived.** The rule keeps a bookmark — `side` and `checked_through`,
the last close read — and `calc::check_alert` reads closes from that bookmark on, re-reading the
last one so a rewritten quote is not missed. Each crossing becomes a row of `alert_crossings` with
the level and close as they were, plus `seen` and `notified`. A daily-close log could be derived
from quotes, but two moves on one day, or a quote taken back, would then erase a crossing the user
was already told about. Editing the level resets the bookmark to today; editing the note does not.
A date rule logs `REACHED` once. `check_alerts` runs after a refresh, after saving a rule, and when
the UI asks for notifications; a rule waiting for a missing rate is skipped, not failed.

**Seen drives the navigation, notified drives the OS.** `alerts_unseen` counts unseen crossings and
the Alerts item carries the same dot a taxonomy tab with gaps does; opening the log — a popup from the
Rules panel — marks it seen. `alerts_take_notifications` checks, then hands over the unannounced crossings, marked
announced in the same transaction. The UI calls it on start, after each refresh and on every
`alerts` change, writes the text and shows it through `tauri-plugin-notification`, asking for
permission itself. Notifications appear only while the app runs.

**A debug build can move the price.** `dev_alert_simulate` writes today's close one percent past the
level on the other side (source `dev`), or deletes those quotes again, and runs the same check. The
log, the dot and the notification are therefore exactly what a real move produces.

**Events ride on the quote request.** `QuoteProvider::fetch_history` returns quotes and events from
one request; its default returns `fetch` and no events. Yahoo adds `events=div|split`.
`MarketDataService` saves reported events next to the quotes and keeps a separate
`event_coverage`; a range counts as fetched only when both hold it, so older databases backfill
events once. Reported events are upserted on `(security_id, kind, date)`. Switching a listing
deletes the reported events with the quotes; the user's notes stay.

**An event moves nothing.** A reported dividend is not income and a reported split is not applied;
the UI offers the split as a corporate action to record and marks it `recorded` once one exists.
Rules, crossings and events are facts about an instrument, so no command takes a scope.

## Alternatives

- **A limit attribute.** Rejected: no per-crossing state, one limit per
  attribute, and a value `> 17` that is text to parse instead of a number and a currency.
- **Firing while the price stays beyond the limit, with a dismiss button.** The first version of
  this ADR. Rejected: "dismiss while beyond" never reports the price coming back.
- **No direction, every crossing logged.** The second version. Rejected: a stop-loss level must not
  announce the recovery above it, and a take-profit level must not announce the fall below it.
- **Derive the log from quotes on every read.** Rejected for the reasons above: a same-day rewrite
  or a deleted quote rewrites history the user was already notified of.
- **The host sends the notification.** Rejected: it would need the language, and the startup refresh
  finishes before the UI can ask for permission.
- **A separate event request.** Rejected: one more request per instrument for data already received.
- **Apply a reported split automatically.** Rejected as ADR-0033 rejects generated purchases.
- **Upcoming ex-dividend dates.** Out of scope: Yahoo's `quoteSummary` needs a cookie and crumb.

## Consequences

- Closes are daily, so two crossings on one real day show as one unless a quote is rewritten.
- The bookmark reads only closes from `checked_through` on; a correction to an older quote is not
  re-examined.
- A background notification with the app closed needs OS scheduling on mobile; nothing here does it.
- The first refresh after migration 0014 re-requests five years of every quoted instrument once.
