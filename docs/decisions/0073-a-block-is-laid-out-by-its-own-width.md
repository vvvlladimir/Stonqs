# 73: A block is laid out by its own width, not by the window's

- Status: Accepted

## Context

The dashboard's columns and several blocks inside its tiles chose their layout from the
**window**: `useBoardColumns` read `window.innerWidth`, `styles/ui/widget.css` picked the board's
column count with `@media`, and `bar.css` switched the target rows between a three-column table
and a stack at a window width of 1000px.

The window is not what those blocks are in. With the assistant panel open the board is half the
screen, so a 600px board was given twelve columns and tiles 40px wide. A 3/12 tile on a wide
screen was given the wide reading of a block that had no room for it, and the same tile filling
a phone row was given the narrow one it did not need. The two readings also disagreed with each
other: the stylesheet asked the window while the drag arithmetic asked the same window for a
board of a different width, so a resize preview and the layout under it could differ.

ADR-0029 stands: a widget still stores one width in twelfths and the board scales it. What
changes is *which width* is scaled onto.

## Decision

Layout is chosen by the box a thing is in.

- The board is a container (`.wboard`) and `useBoardColumns(ref)` measures the **grid element**
  with a `ResizeObserver`. The stylesheet reads the same width through `@container board`, so the
  columns drawn and the columns counted are one number.
- Every tile is a container (`container-type: size`, so both axes can be asked). Density — the
  header icon, the period chip, the padding, the list-row shape — is a function of the tile.
- **A block that reshapes by width declares its own container**: `.wfall`, `.drift`, `.calbox`.
  The rule then holds wherever the block is used — a widget tile, a panel on a screen, a dialog —
  and no caller declares anything.
- On a two-column board a widget whose catalog `min.w` is 4/12 or more spans the row
  (`grid.shownSpan`). Every chart declares such a minimum, and a plot 150px wide is a texture,
  not a reading. The rule is read off the catalog rather than from a new "wide on mobile" flag.

## Alternatives

- **Per-breakpoint layouts stored per widget.** Three layouts to keep in step and to export;
  rejected for the same reason ADR-0029 rejected it.
- **Keep `@media` and pass the board width down as a CSS variable.** Works for the grid, not for
  a block used on a screen where no one sets the variable.
- **A `wide` flag in the widget catalog.** A second place to state what `min.w` already says,
  and one that would be forgotten by the next widget.

## Consequences

- A container query measures the **content box**, so every threshold in these stylesheets is the
  box's width minus its padding. This is stated where the thresholds are.
- `container-type: size` makes a tile's height independent of its content, which the grid already
  guaranteed; a plain (section-heading) tile is sized *by* its content and keeps `inline-size`.
- The assistant panel, a future split view and the phone all reflow correctly without new rules.
- `app/mockups/` renders the real stylesheets, so a layout can be checked at any board width
  without launching the desktop app.
- The phone rule's "read off the catalog's own `min.w`" is narrowed by ADR-0074: the width it
  reads is the widget's own when the board's owner named one, and the catalog's otherwise.
