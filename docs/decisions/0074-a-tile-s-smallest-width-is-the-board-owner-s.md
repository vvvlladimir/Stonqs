# 74: A tile's smallest width is the board owner's

- Status: Accepted

## Context

ADR-0073 made the phone reading of a tile a function of the catalog: on a two-column board a
widget whose `min.w` is 4/12 or more takes the whole row, and the rule is read off `min.w` "rather
than from a new *wide on mobile* flag".

That is the right default and the wrong final answer. The catalog states what a widget reads at in
general; it cannot know that this user's watchlist of three tickers is unreadable at half a phone
column, or that their composition tile is fine there. The board is already the user's — they
choose the tiles, the widths, the order and the periods — and the one number deciding whether a
tile is legible on their phone was the only part of it they could not touch.

## Decision

A widget may carry a minimum width of its own, `cfg.min_w`, in twelfths like every other width.
`grid.limitsOf(widget, def)` resolves it: the widget's, else the catalog's `min.w`. That one
number is where a resize drag stops **and** what `shownSpan` asks on a two-column board, so
raising it to 4 is how a user says "this one takes the row on a phone".

The setting is offered by every non-plain widget's dialog and is therefore **not** listed in
`WidgetDef::fields`: how small a tile may get is a property of the board, not of what the tile
shows, so a new widget gains it without a line in the catalog. Saving raises the tile's stored
width to the minimum, because a minimum nothing enforces is not one.

## Alternatives

- **Leave it to the catalog (ADR-0073 as it stood).** One number per widget type for every user
  and every board; the only escape was editing the exported layout file by hand.
- **A "wide on mobile" switch.** The flag ADR-0073 rejected, and it answers one board width only —
  a tablet board has six columns and the switch says nothing about them.
- **Per-breakpoint widths.** Rejected twice already, by ADR-0029 and ADR-0073, for the same
  reason: three layouts to keep in step and three to export.

## Consequences

- ADR-0073's fourth bullet is narrowed, not reversed: the phone rule is still read off a minimum
  width, and that minimum now has a user's answer ahead of the catalog's.
- A board file carries the choice, so an exported layout lays out the same way on another machine.
- The catalog's `min.h` is untouched: a tile too short is scrolled by its own body, which is not
  the failure a tile too narrow is.
