# 59: A tile may hold assumptions, and a ratio of two shown figures is a rendering

- Status: Accepted

## Context

Two dashboard widgets found in other trackers do not fit the shape every widget here has had so
far — "ask the host for a figure it computed and print it".

The **FIRE** tile needs numbers nobody measured: a yearly spending, a withdrawal rate, an expected
return. The **Ratio** tile needs no new figure at all; it divides two the dashboard already shows.

## Decision

**FIRE is a calculation, and its inputs are the user's.** `calc::fire_projection` takes today's
value and a `FireAssumptions` and answers a target, a progress and a horizon. It never reads this
portfolio's measured return: the question is "what would it take", not "what happened", and a tile
that quietly fed last year's return into a thirty-year projection would be inventing a forecast.
Whether the return given is real or nominal is likewise the user's business and is documented, not
guessed.

The horizon is solved in `f64` (a logarithm, the same exception XIRR and volatility already take)
and rounded **up** to a whole month, because a target half-reached is not reached. It is `None`
beyond a hundred years, including the cases that never arrive at all.

The tile is **not scoped**. The contribution comes from the investment plans, which belong to the
portfolio and not to a lens (ADR-0033); reading one account's value against every account's
savings would compare two different things.

**A ratio is drawn, not computed.** The Ratio widget divides two figures already on the wire, in
the frontend, because the result is a share for display — nothing is added, rounded into money, or
stored. This is the same licence `Income` already takes for its portfolio yield and its tax share.
Both terms come from queries the dashboard makes anyway (`dashboard_summary`, `period_summary`,
`income_summary`), so a ratio tile beside a metric tile costs no extra request, and a divisor of
zero renders as absent rather than as zero.

## Alternatives

- **A `ratio_metric` command with a named-term catalogue in the host.** Keeps every division in
  Rust, but duplicates the term list on both sides of the boundary and adds a request per tile to
  divide two numbers the screen is already holding.
- **Feeding the portfolio's own past return into FIRE as the default.** Reads as authoritative
  precisely where it is least defensible: a measured five-year return is not a thirty-year
  assumption.
- **Storing the assumptions in `AppSettings`.** They belong to the tile that asks the question —
  two tiles may ask it with different spending, as two chats hold two providers.

## Consequences

- The FIRE tile shows nothing until it is told what a year costs; an empty spending is a missing
  assumption, not a zero.
- Every ratio the user can build is a pair from one fixed list, so a term that needs a new figure
  needs a host change — which is the point: the list is what the dashboard already knows.
- Changing a plan invalidates the FIRE reading along with the rest of the plans group.
