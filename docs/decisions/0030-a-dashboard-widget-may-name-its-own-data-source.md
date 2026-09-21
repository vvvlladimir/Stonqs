# 30: A dashboard widget may name its own data source

- Status: Accepted

## Context

The data source is the app's lens: `AppState::scope` holds one `DataScope`, every reporting
command answers in it, and the picker in the navigation changes it for the whole app at once
(see `.claude/rules/ui-boundary.md`). That is right for a screen — the positions table, the
performance chart and the risk report should all be talking about the same money.

A dashboard is not one screen. A board is assembled by the user out of tiles, and the obvious
boards are the ones a single lens cannot draw: the whole portfolio's value beside one broker's
positions, a cash tile per account, two "Return (TWR)" tiles comparing a pension account with a
trading one. With one global scope such a board says the same thing four times.

## Decision

A reporting command takes an optional `source: Option<DataScope>` and resolves it through
`AppState::scope_selection_in(store, source)`. Absent — which is what every screen sends — means
the scope the picker holds, so nothing about the existing screens changes. The scope is still
host state; what is new is that a *request* may carry one.

On the frontend the argument travels the same way as every other: `lib/api.ts` takes
`source?: Source`, the query hook takes it last and puts it last in its key. `keys.positions()`
is therefore still a prefix of every positions key, and `invalidate(...affects.positions)` still
covers every source on the board.

A widget stores its choice in `cfg.source` as the wire shape (`{ kind, id }`) and the catalog
lists `source` among its `fields`, so the shared configuration dialog offers it beside the
widget's own settings. `sourceOf` reads it back; anything unrecognised reads as absent, so a
board naming an account that no longer exists degrades to the picker rather than failing.

Two commands deliberately keep the global scope:

- `app_status` is the application's own state, not a report.
- `period_ranges` is the board's axis. Every tile of a dashboard shares the period strip above
  it, and "since inception" resolving to a different day per tile would make two tiles labelled
  the same incomparable. The source narrows *what* is measured, never *when*.

A report export also passes `None`: it is an export of what the screen shows.

## Alternatives

- **A scope per dashboard rather than per widget.** Cheaper, and it answers "a board for this
  account" — but not "this tile against that tile", which is the comparison that makes the
  feature worth having.
- **Widen `AppState::scope` into a stack the widget pushes onto.** The lock is held across a
  whole command already; a per-request argument is the same thing without the mutable state.
- **Let the frontend narrow the answer.** It cannot: the narrowing is `calc::scoped_transactions`
  rewriting the leg that falls outside the scope, and the frontend has neither the transactions
  nor the right to do arithmetic on them.

## Consequences

- Every board is potentially several calculations of the same shape. They share a cache key per
  source, so two tiles on one source still cost one pass, but a board with six sources pays for
  six. That is the user's choice to make and it is visible in what they assembled.
- A tile pinned to a source does not follow the picker, which is the point and also a way to be
  confused. The configuration dialog says so in as many words; the tile header does not repeat
  it, so a board of pinned tiles is read by opening one.
- The scope argument is now part of the wire format of sixteen commands. A new reporting command
  is expected to take it; one that forgets simply cannot be pointed at an account.
