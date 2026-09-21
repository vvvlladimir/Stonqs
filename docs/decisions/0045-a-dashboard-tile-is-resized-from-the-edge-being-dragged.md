# 45: A dashboard tile is resized from the edge being dragged

- Status: Accepted

## Context

ADR-0029 made the board a flow of tiles — order plus size, no coordinates — and let every edge
resize a tile. Because a flow has no far edge to pin, the top and left handles changed the size
only: dragging the top edge up grew the tile *downwards*, and dragging the left edge right shrank
it from the right. The edge under the pointer did not follow the pointer, which reads as a bug.

## Decision

The edge being dragged is the one that moves; the opposite one stays. A widget gains two optional
fields, both absent on every board written before this:

- `x` — the first column, in twelfths like `w`. Set by the left handle as `end - width`, so the
  right edge stays. The flow still places the tile: CSS grid's sparse auto-placement treats a
  definite column start as "the next row where that column is free, in document order", so order
  is kept and nothing collides. On a narrower board `x` scales like `w` and is clamped so the tile
  stays on the board.
- `y` — empty rows kept above the tile *inside its own slot* (`grid-row: span h + y`, the tile
  itself pushed down by `margin-top`). Set by the top handle as `bottom - height`, so the bottom
  edge stays.

The left edge may grow only as far as the nearest tile beside it in any row it spans, measured at
the press; the top edge only as far as the top of its own slot. Past that the flow would have to
wrap the tile to another row or take rows from the tile above — neither is something the pointer
asked for. Moving a tile clears both fields: a pinned column and an empty strip belonged to the
place it left, and keeping them would land it somewhere other than where it was dropped.

`UI_VERSION` is not bumped: both fields are optional, and an older reader drops them and gets the
flow it always had.

## Alternatives

- **Free `x`/`y` coordinates.** Rejected for the same reasons as in ADR-0029: collision
  resolution and compaction, reapplied on every column count.
- **Pin the far edge only while dragging**, and let the flow place the tile on release. The tile
  would jump away from the pointer at the moment the user lets go.
- **Let the top edge push the tiles above it down.** That is a reorder, which is the header's
  gesture; one handle doing both makes neither predictable.

## Consequences

- A tile at the very top of its slot cannot be grown upwards by its top edge; the bottom edge
  does that. A tile at the start of a row likewise cannot grow leftwards.
- The grid gap is a named variable (`--w-gap`) because an empty row above a tile costs a row and
  a gap.
