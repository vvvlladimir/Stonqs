# 14: Market refresh is one background job broadcasting a global event


- Status: Accepted

## Context

Refreshing quotes and FX rates means tens of seconds of network calls. It can
be triggered by app startup and by a button in settings, and a screen showing
progress may open after the refresh has already started.

## Decision

`app/src-tauri/src/jobs.rs` runs refresh as a single job for the whole app,
not one per caller.

- A second trigger while one is running is a no-op, not an error — the user
  pressed the button while auto-refresh was already working.
- The job opens its **own** `Store` connection on its own thread (WAL makes
  that safe) so `Mutex<Store>` is never held across a network call.
- Progress is a global `market:progress` event, not a channel scoped to the
  call that started the job: a channel is tied to its caller, and the
  caller that matters most — app startup — has no UI listening yet. A
  screen opening mid-refresh reads current progress via `refresh_status`
  instead.
- Each security is saved right after its own fetch, not batched at the end:
  an interrupted job keeps whatever it already fetched, and `quote_coverage`
  extending even on an empty response (see `money-and-fx.md`) is what stops
  a closed-market day from being re-requested forever.
- The job body runs under `catch_unwind`. `finish` must execute on every
  outcome because it's what clears the `running` flag; without the guard, a
  panic mid-job would leave both refresh buttons dead until the app
  restarts. Nothing is reused after a panic — `Store` and the provider
  services are rebuilt from scratch inside `run`.
